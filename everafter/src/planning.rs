use rusqlite::{params,Connection};
use serde_json::{json,Value};
use std::collections::{HashMap,HashSet};
use axum::{extract::State,http::StatusCode,Json};
use crate::{Db,ApiResult,error};

pub fn list(db:&Connection,kind:&str)->Result<Vec<Value>,String>{let mut s=db.prepare("SELECT data FROM records WHERE kind=?1 ORDER BY rowid").map_err(|e|e.to_string())?;let rows=s.query_map([kind],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;rows.map(|r|serde_json::from_str(&r.map_err(|e|e.to_string())?).map_err(|e|e.to_string())).collect()}
fn text<'a>(v:&'a Value,k:&str)->&'a str{v[k].as_str().unwrap_or("")}
fn number(v:&Value,k:&str)->f64{v[k].as_f64().unwrap_or(0.0)}
fn ids(v:&Value)->Vec<&str>{v["dependencyIds"].as_array().map(|a|a.iter().filter_map(Value::as_str).collect()).unwrap_or_default()}
pub fn room_labels(booking:&Value)->Vec<String>{let explicit=text(booking,"roomLabels").split([',','\n']).map(str::trim).filter(|v|!v.is_empty()).map(str::to_owned).collect::<Vec<_>>();if !explicit.is_empty(){explicit}else{(1..=booking["roomCount"].as_u64().unwrap_or(0)).map(|n|format!("Room {n}")).collect()}}
pub fn validate(db:&Connection,kind:&str,id:&str,record:&Value)->Result<(),String>{
    if kind=="tasks" {
        if record.get("done").is_some_and(|v|!v.is_boolean())||record.get("dependencyIds").is_some_and(|v|v.as_array().is_none_or(|a|a.len()>1000||a.iter().any(|id|!id.is_string()))){return Err("Task completion must be a boolean and dependencies must be task IDs.".into())}
        let mut tasks=list(db,"tasks")?;tasks.retain(|t|text(t,"id")!=id);tasks.push(record.clone());
        let map:HashMap<&str,&Value>=tasks.iter().map(|t|(text(t,"id"),t)).collect();
        fn visit<'a>(id:&'a str,map:&HashMap<&'a str,&'a Value>,visiting:&mut HashSet<&'a str>,done:&mut HashSet<&'a str>)->Result<(),String>{if done.contains(id){return Ok(())}if !visiting.insert(id){return Err("Task dependencies cannot form a loop.".into())}let t=map.get(id).ok_or("A prerequisite no longer exists.")?;for dep in ids(t){visit(dep,map,visiting,done)?;}visiting.remove(id);done.insert(id);Ok(())}
        for key in map.keys(){visit(key,&map,&mut HashSet::new(),&mut HashSet::new())?;}
        for task in &tasks {if task["done"]==true && ids(task).iter().any(|d|map.get(d).map(|t|t["done"]!=true).unwrap_or(true)){return Err("Complete prerequisites first, or reopen dependent tasks before reopening this one.".into());}}
    }
    if kind=="settings" {
        if record.get("budget").is_some() && (!number(record,"budget").is_finite()||number(record,"budget")<0.0){return Err("Budget cannot be negative.".into())}
        if let Some(items)=record["budgetAllocations"].as_array(){let mut seen=HashSet::new();let mut sum=0.0;for item in items{if text(item,"id").is_empty()||!seen.insert(text(item,"id")){return Err("Budget activities must have unique IDs.".into())}let p=number(item,"percent");if !p.is_finite()||!(0.0..=100.0).contains(&p){return Err("Each earmark must be between 0 and 100%.".into())}sum+=p;}if sum>100.0001{return Err("Budget earmarks cannot exceed 100%.".into());}}
    }
    if ["bookings","expenses","vendors"].contains(&kind){for field in if kind=="bookings"{vec!["totalPrice","paid"]}else if kind=="vendors"{vec!["quote","paid"]}else{vec!["amount","paid"]}{if number(record,field)<0.0{return Err("Costs and payments cannot be negative.".into())}}let total=number(record,if kind=="bookings"{"totalPrice"}else if kind=="vendors"{"quote"}else{"amount"});if number(record,"paid")>total{return Err("Paid amount cannot exceed the recorded cost.".into())}}
    if kind=="events" {if text(record,"name").trim().is_empty()||text(record,"date").is_empty()||text(record,"venue").trim().is_empty(){return Err("Add an event name, date, and venue.".into())}if !text(record,"endTime").is_empty()&&text(record,"endTime")<text(record,"time"){return Err("End time must follow start time (use a separate event for overnight schedules).".into())}}
    if kind=="bookings" {
        let count=record["roomCount"].as_u64().unwrap_or(0);let beds=record["capacityPerRoom"].as_u64().unwrap_or(0);
        if count==0||count>1000||beds==0||beds>20{return Err("Use 1–1,000 rooms and 1–20 guests per room.".into())}
        let labels=room_labels(record);if labels.len()!=count as usize||labels.iter().collect::<HashSet<_>>().len()!=labels.len(){return Err("Provide exactly one unique room label per booked room, or leave labels blank.".into())}
        if text(record,"checkIn").is_empty()||text(record,"checkOut")<=text(record,"checkIn"){return Err("Checkout must be after check-in.".into())}
        let allocations=list(db,"allocations")?;
        for a in allocations.iter().filter(|a|text(a,"bookingId")==id){if !labels.iter().any(|r|r==text(a,"room")){return Err("Reassign guests before removing a room label.".into())}let used:f64=allocations.iter().filter(|x|text(x,"bookingId")==id&&text(x,"room")==text(a,"room")).map(|x|number(x,"partySize")).sum();if used>beds as f64{return Err("Reassign guests before reducing room capacity.".into())}}
    }
    if kind=="allocations" {
        let bookings=list(db,"bookings")?;let booking=bookings.iter().find(|b|text(b,"id")==text(record,"bookingId")).ok_or("Choose an existing booking.")?;
        let guests=list(db,"guests")?;let guest=guests.iter().find(|g|text(g,"id")==text(record,"guestId")).ok_or("Choose an existing guest.")?;
        if text(guest,"rsvp")=="Declined"{return Err("This guest has declined. Update their RSVP before allocating a room.".into())}
        if !room_labels(booking).iter().any(|r|r==text(record,"room")){return Err("Choose a room belonging to this booking.".into())}
        let party=record["partySize"].as_u64().unwrap_or(0);if party==0{return Err("Party size must be at least one.".into())}
        let allocations=list(db,"allocations")?;if allocations.iter().any(|a|text(a,"id")!=id&&text(a,"guestId")==text(record,"guestId")){return Err("This guest already has a room. Edit that allocation first.".into())}
        let occupied:f64=allocations.iter().filter(|a|text(a,"id")!=id&&text(a,"bookingId")==text(record,"bookingId")&&text(a,"room")==text(record,"room")).map(|a|number(a,"partySize")).sum();
        if occupied+party as f64>number(booking,"capacityPerRoom"){return Err("This room does not have enough capacity.".into())}
    }
    if kind=="requests" {
        if text(record,"pickupLocation").trim().is_empty()||text(record,"dropLocation").trim().is_empty()||text(record,"arrivalTime").is_empty(){return Err("Add pickup, drop-off, and arrival time.".into())}
        if record["passengers"].as_u64().filter(|n|*n>=1&&*n<=100).is_none(){return Err("Use a passenger count from 1 to 100.".into())}
        if !text(record,"rideId").is_empty() && text(record,"status")!="Cancelled" {
            let rides=list(db,"rides")?;let ride=rides.iter().find(|r|text(r,"id")==text(record,"rideId")).ok_or("Choose an existing vehicle.")?;
            // A vehicle record represents one pickup run. Completed/cancelled requests release capacity.
            let used:f64=list(db,"requests")?.iter().filter(|r|text(r,"id")!=id&&text(r,"rideId")==text(record,"rideId")&&!["Dropped off","Cancelled"].contains(&text(r,"status"))).map(|r|number(r,"passengers")).sum();
            if !["Dropped off","Cancelled"].contains(&text(record,"status")) && used+number(record,"passengers")>number(ride,"capacity"){return Err("This pickup run does not have enough seats.".into())}
        }
    }
    Ok(())
}
pub fn check_delete(db:&Connection,kind:&str,id:&str)->Result<(),String>{
    let referenced=match kind {"tasks"=>list(db,"tasks")?.iter().any(|t|ids(t).contains(&id)),"bookings"=>list(db,"allocations")?.iter().any(|a|text(a,"bookingId")==id),"guests"=>list(db,"allocations")?.iter().any(|a|text(a,"guestId")==id)||list(db,"requests")?.iter().any(|r|text(r,"guestId")==id),"rides"=>list(db,"requests")?.iter().any(|r|text(r,"rideId")==id),_=>false};
    if referenced{Err("This record is in use. Remove its dependencies or reassign its guests first.".into())}else{Ok(())}
}
pub fn sync_allocation(db:&Connection,guest_id:&str)->Result<(),String>{
    let guest=list(db,"guests")?.into_iter().find(|g|text(g,"id")==guest_id);if let Some(mut g)=guest {
        if let Some(a)=list(db,"allocations")?.iter().find(|a|text(a,"guestId")==guest_id){let bookings=list(db,"bookings")?;if let Some(b)=bookings.iter().find(|b|text(b,"id")==text(a,"bookingId")){g["hotel"]=b["name"].clone();g["room"]=a["room"].clone();g["hotelAddress"]=b["address"].clone();g["hotelAllocationId"]=a["id"].clone();}}
        else if g.get("hotelAllocationId").is_some(){for k in ["hotel","room","hotelAddress","hotelAllocationId"]{g[k]=json!("");}}
        db.execute("UPDATE records SET data=?1 WHERE kind='guests' AND id=?2",params![g.to_string(),guest_id]).map_err(|e|e.to_string())?;
    }Ok(())
}
pub fn after_save(db:&Connection,kind:&str,record:&Value)->Result<(),String>{
    if kind=="allocations"{sync_allocation(db,text(record,"guestId"))?;}
    if kind=="bookings"{for a in list(db,"allocations")?.iter().filter(|a|text(a,"bookingId")==text(record,"id")){sync_allocation(db,text(a,"guestId"))?;}}
    if kind=="events"&&record["isMain"]==true{
        db.execute("UPDATE records SET data=json_set(data,'$.isMain',json('false')) WHERE kind='events' AND id!=?1",[text(record,"id")]).map_err(|e|e.to_string())?;
        if let Some(mut s)=list(db,"settings")?.into_iter().next(){s["date"]=record["date"].clone();s["venue"]=record["venue"].clone();s["time"]=record["time"].clone();db.execute("UPDATE records SET data=?1 WHERE kind='settings' AND id=?2",params![s.to_string(),text(&s,"id")]).map_err(|e|e.to_string())?;}
    }Ok(())
}
pub async fn auto_allocate(State(db):State<Db>)->ApiResult{
    let mut db=db.lock().map_err(error)?;let tx=db.transaction().map_err(error)?;
    let bookings=list(&tx,"bookings").map_err(error)?;let tiers=list(&tx,"tiers").map_err(error)?;let existing=list(&tx,"allocations").map_err(error)?;
    let mut guests=list(&tx,"guests").map_err(error)?.into_iter().filter(|g|g["needsHotel"]==true&&text(g,"rsvp")=="Attending"&&!existing.iter().any(|a|text(a,"guestId")==text(g,"id"))).collect::<Vec<_>>();
    guests.sort_by_key(|g|tiers.iter().find(|t|text(t,"name")==text(g,"tier")).and_then(|t|t["rank"].as_u64()).unwrap_or(999));
    let mut used=existing.iter().map(|a|(text(a,"bookingId").to_owned(),text(a,"room").to_owned())).collect::<HashSet<_>>();let mut assigned=0;let mut waiting=Vec::new();
    for guest in guests{let party=guest["partySize"].as_u64().unwrap_or(1).max(1);let preference=text(&guest,"roomPreference");let mut matched=false;
        for booking in &bookings{if (!text(&guest,"stayFrom").is_empty()&&text(booking,"checkIn")>text(&guest,"stayFrom"))||(!text(&guest,"stayTo").is_empty()&&text(booking,"checkOut")<text(&guest,"stayTo")){continue}if number(booking,"capacityPerRoom")<party as f64{continue}let eligible=booking["eligibleTiers"].as_array();if eligible.is_some_and(|a|!a.is_empty()&&!a.iter().any(|t|t.as_str()==Some(text(&guest,"tier")))){continue}if !preference.is_empty()&&preference!="Any"&&preference!=text(booking,"roomType"){continue}
            if let Some(room)=room_labels(booking).into_iter().find(|r|!used.contains(&(text(booking,"id").to_owned(),r.clone()))){let id=uuid::Uuid::new_v4().to_string();let a=json!({"id":id,"bookingId":booking["id"],"guestId":guest["id"],"room":room,"partySize":party,"source":"Auto allocation"});tx.execute("INSERT INTO records(kind,id,data) VALUES('allocations',?1,?2)",params![id,a.to_string()]).map_err(error)?;sync_allocation(&tx,text(&guest,"id")).map_err(error)?;used.insert((text(booking,"id").to_owned(),room));assigned+=1;matched=true;break}
        }if !matched{waiting.push(json!({"id":guest["id"],"name":guest["name"]}));}
    }
    tx.commit().map_err(error)?;Ok(Json(json!({"assigned":assigned,"waiting":waiting})))
}
pub async fn assign_transport(State(db):State<Db>,Json(body):Json<Value>)->ApiResult{
    let ids=body["ids"].as_array().filter(|v|!v.is_empty()&&v.len()<=1000).ok_or((StatusCode::BAD_REQUEST,"Select an itinerary to assign.".into()))?;
    let ride=body["rideId"].as_str().unwrap_or("");let mut db=db.lock().map_err(error)?;let tx=db.transaction().map_err(error)?;
    for id in ids{let requests=list(&tx,"requests").map_err(error)?;let mut r=requests.into_iter().find(|r|r["id"]==*id).ok_or((StatusCode::CONFLICT,"A request was removed. Refresh and try again.".into()))?;r["rideId"]=json!(ride);r["status"]=json!(if ride.is_empty(){"Waiting"}else{"Assigned"});validate(&tx,"requests",text(&r,"id"),&r).map_err(|m|(StatusCode::BAD_REQUEST,m))?;tx.execute("UPDATE records SET data=?1 WHERE kind='requests' AND id=?2",params![r.to_string(),text(&r,"id")]).map_err(error)?;}
    tx.commit().map_err(error)?;Ok(Json(json!({"ok":true})))
}

#[cfg(test)]mod tests{
use super::*;fn db()->Connection{let d=Connection::open_in_memory().unwrap();d.execute_batch("CREATE TABLE records(kind TEXT,id TEXT,data TEXT,PRIMARY KEY(kind,id));").unwrap();d}fn put(d:&Connection,k:&str,v:Value){d.execute("INSERT INTO records VALUES(?1,?2,?3)",params![k,text(&v,"id"),v.to_string()]).unwrap();}
#[test]fn dependencies_block_cycles_and_early_completion(){let d=db();put(&d,"tasks",json!({"id":"a","done":false}));put(&d,"tasks",json!({"id":"b","dependencyIds":["a"],"done":false}));assert!(validate(&d,"tasks","a",&json!({"id":"a","dependencyIds":["b"]})).is_err());assert!(validate(&d,"tasks","b",&json!({"id":"b","dependencyIds":["a"],"done":true})).is_err());}
#[test]fn budget_percentages_and_booked_room_capacity_are_checked(){let d=db();assert!(validate(&d,"settings","s",&json!({"budgetAllocations":[{"id":"a","percent":60},{"id":"b","percent":50}]})).is_err());put(&d,"bookings",json!({"id":"b","roomCount":1,"capacityPerRoom":2}));put(&d,"guests",json!({"id":"g","rsvp":"Attending"}));assert!(validate(&d,"allocations","a",&json!({"id":"a","bookingId":"b","guestId":"g","room":"Room 1","partySize":3})).is_err());}
#[tokio::test]async fn auto_allocation_uses_tier_priority_and_preserves_existing(){let d=db();put(&d,"tiers",json!({"id":"t1","name":"First","rank":1}));put(&d,"tiers",json!({"id":"t2","name":"Second","rank":2}));put(&d,"bookings",json!({"id":"b","name":"Hotel","roomCount":1,"capacityPerRoom":2,"roomType":"Double","checkIn":"2027-03-01","checkOut":"2027-03-03"}));put(&d,"guests",json!({"id":"later","name":"Later priority","tier":"Second","rsvp":"Attending","needsHotel":true,"partySize":1}));put(&d,"guests",json!({"id":"first","name":"First priority","tier":"First","rsvp":"Attending","needsHotel":true,"partySize":2,"roomPreference":"Double","stayFrom":"2027-03-01","stayTo":"2027-03-03"}));let d=std::sync::Arc::new(std::sync::Mutex::new(d));let result=auto_allocate(State(d.clone())).await.unwrap().0;assert_eq!(result["assigned"],1);assert_eq!(list(&d.lock().unwrap(),"allocations").unwrap()[0]["guestId"],"first");assert_eq!(auto_allocate(State(d.clone())).await.unwrap().0["assigned"],0);assert_eq!(list(&d.lock().unwrap(),"guests").unwrap().iter().find(|g|g["id"]=="first").unwrap()["hotel"],"Hotel");}
#[tokio::test]async fn grouped_vehicle_assignment_rolls_back_if_seats_run_out(){let d=db();put(&d,"rides",json!({"id":"r","capacity":2}));for id in ["q1","q2"]{put(&d,"requests",json!({"id":id,"pickupLocation":"Airport","dropLocation":"Hotel","arrivalTime":"2027-03-01T12:00","passengers":2,"status":"Waiting"}));}let d=std::sync::Arc::new(std::sync::Mutex::new(d));assert!(assign_transport(State(d.clone()),Json(json!({"ids":["q1","q2"],"rideId":"r"}))).await.is_err());assert!(list(&d.lock().unwrap(),"requests").unwrap().iter().all(|r|r["rideId"].is_null()));}
}
