use axum::{extract::State,http::StatusCode,Json};
use rusqlite::{params,Connection};
use serde_json::{json,Value};
use crate::{Db,ApiResult,error};

pub fn initialize(db:&Connection)->rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS organization_initialized(id INTEGER PRIMARY KEY);")?;
    let first=db.execute("INSERT OR IGNORE INTO organization_initialized(id) VALUES(1)",[])?;
    if first>0 {
        for (id,name) in [("family","Family"),("friends","Friends"),("colleagues","Colleagues"),("other","Other")] {
            let record=json!({"id":id,"name":name,"description":""});
            db.execute("INSERT OR IGNORE INTO records(kind,id,data) VALUES('categories',?1,?2)",params![id,record.to_string()])?;
        }
        for (id,name,rank,hotel,travel,description) in [
            ("inner-circle","Tier 1 · Inner circle",1,true,true,"Close family and friends you plan to host and help with travel."),
            ("extended-circle","Tier 2 · Extended circle",2,true,false,"Guests you plan to offer accommodation."),
            ("wider-circle","Tier 3 · Wider circle",3,false,false,"Guests coordinating their own stay and travel.")
        ] {
            let record=json!({"id":id,"name":name,"rank":rank,"hotelSupport":hotel,"travelSupport":travel,"description":description});
            db.execute("INSERT OR IGNORE INTO records(kind,id,data) VALUES('tiers',?1,?2)",params![id,record.to_string()])?;
        }
    }
    // Preserve labels created before managed categories/tiers existed, including CSV imports.
    for (kind,field) in [("tiers","tier"),("categories","category")] {
        let mut stmt=db.prepare("SELECT DISTINCT json_extract(data,?1) FROM records WHERE kind='guests'")?;
        let labels=stmt.query_map([format!("$.{field}")],|r|r.get::<_,Option<String>>(0))?.collect::<Result<Vec<_>,_>>()?;
        for name in labels.into_iter().flatten().filter(|s|!s.trim().is_empty()) {ensure_label(db,kind,&name)?;}
    }
    Ok(())
}
fn ensure_label(db:&Connection,kind:&str,name:&str)->rusqlite::Result<()> {
    let exists:i64=db.query_row("SELECT count(*) FROM records WHERE kind=?1 AND lower(json_extract(data,'$.name'))=lower(?2)",params![kind,name],|r|r.get(0))?;
    if exists==0 {let id=uuid::Uuid::new_v4().to_string();let record=json!({"id":id,"name":name,"description":"Imported guest label","rank":99,"hotelSupport":false,"travelSupport":false});db.execute("INSERT INTO records(kind,id,data) VALUES(?1,?2,?3)",params![kind,id,record.to_string()])?;}
    Ok(())
}
pub fn validate_and_rename(db:&Connection,kind:&str,id:&str,data:&mut Value)->Result<(),String> {
    if kind=="guests" {
        for (group,field) in [("tiers","tier"),("categories","category")] {
            if let Some(name)=data.get(field).and_then(Value::as_str).filter(|s|!s.trim().is_empty()).map(|s|s.trim().to_owned()) {
                if name.len()>120 {return Err("Category and tier names must be under 120 characters.".into());}
                ensure_label(db,group,&name).map_err(|e|e.to_string())?;
                let canonical:String=db.query_row("SELECT json_extract(data,'$.name') FROM records WHERE kind=?1 AND lower(json_extract(data,'$.name'))=lower(?2) LIMIT 1",params![group,name],|r|r.get(0)).map_err(|e|e.to_string())?;
                data[field]=json!(canonical);
            }
        }
    }
    if kind!="categories" && kind!="tiers" {return Ok(());}
    let name=data.get("name").and_then(Value::as_str).unwrap_or("").trim().to_string();
    if name.is_empty() || name.len()>120 {return Err("Enter a name between 1 and 120 characters.".into());}
    if kind=="tiers" && data.get("rank").and_then(Value::as_u64).filter(|n|*n>=1&&*n<=999).is_none() {return Err("Tier priority must be a whole number from 1 to 999.".into());}
    let duplicates:i64=db.query_row("SELECT count(*) FROM records WHERE kind=?1 AND id!=?2 AND lower(json_extract(data,'$.name'))=lower(?3)",params![kind,id,name],|r|r.get(0)).map_err(|e|e.to_string())?;
    if duplicates>0 {return Err("That name already exists. Choose a different name.".into());}
    let old:Option<String>=db.query_row("SELECT json_extract(data,'$.name') FROM records WHERE kind=?1 AND id=?2",params![kind,id],|r|r.get(0)).ok();
    if let Some(old)=old {let field=if kind=="tiers"{"$.tier"}else{"$.category"};db.execute("UPDATE records SET data=json_set(data,?1,?2) WHERE kind='guests' AND json_extract(data,?1)=?3",params![field,name,old]).map_err(|e|e.to_string())?;}
    data["name"]=json!(name); Ok(())
}
pub fn check_delete(db:&Connection,kind:&str,id:&str)->Result<(),String> {
    if kind!="tiers" && kind!="categories" {return Ok(());}
    let name:Option<String>=db.query_row("SELECT json_extract(data,'$.name') FROM records WHERE kind=?1 AND id=?2",params![kind,id],|r|r.get(0)).ok();
    if let Some(name)=name {let field=if kind=="tiers"{"$.tier"}else{"$.category"};let count:i64=db.query_row("SELECT count(*) FROM records WHERE kind='guests' AND json_extract(data,?1)=?2",params![field,name],|r|r.get(0)).map_err(|e|e.to_string())?;if count>0{return Err("Reassign guests before deleting this group.".into());}}
    Ok(())
}
pub async fn bulk(State(db):State<Db>,Json(body):Json<Value>)->ApiResult {
    let ids=body.get("ids").and_then(Value::as_array).filter(|v|!v.is_empty()&&v.len()<=5000).ok_or((StatusCode::BAD_REQUEST,"Select at least one guest (up to 5,000).".into()))?;
    let mut db=db.lock().map_err(error)?;let tx=db.transaction().map_err(error)?;
    let mut fields=Vec::new();
    for (field,kind) in [("category","categories"),("tier","tiers")] {
        if let Some(value)=body.get(field) {let name=value.as_str().ok_or((StatusCode::BAD_REQUEST,"Invalid group name.".into()))?;
            let exists:i64=tx.query_row("SELECT count(*) FROM records WHERE kind=?1 AND json_extract(data,'$.name')=?2",params![kind,name],|r|r.get(0)).map_err(error)?;
            if !name.is_empty()&&exists==0{return Err((StatusCode::BAD_REQUEST,"That group no longer exists. Refresh and try again.".into()));}
            fields.push((format!("$.{field}"),name));
        }
    }
    if fields.is_empty(){return Err((StatusCode::BAD_REQUEST,"Choose a category or tier to apply.".into()));}
    for id in ids {let id=id.as_str().ok_or((StatusCode::BAD_REQUEST,"Invalid guest ID.".into()))?;
        for (field,value) in &fields {let changed=tx.execute("UPDATE records SET data=json_set(data,?1,?2) WHERE kind='guests' AND id=?3",params![field,value,id]).map_err(error)?;if changed==0{return Err((StatusCode::CONFLICT,"A selected guest was removed. Refresh and try again.".into()));}if field=="$.category"{tx.execute("UPDATE records SET data=json_set(data,'$.categoryReview','confirmed') WHERE kind='guests' AND id=?1",[id]).map_err(error)?;}}
    }
    tx.commit().map_err(error)?;Ok(Json(json!({"ok":true})))
}
fn contact_key(value:&str,phone:bool)->String {
    if phone {let digits:String=value.chars().filter(char::is_ascii_digit).collect();digits.strip_prefix("00").unwrap_or(&digits).to_owned()}
    else {value.trim().to_lowercase()}
}
pub async fn import_contacts(State(db):State<Db>,Json(body):Json<Value>)->ApiResult {
    let contacts=body.get("contacts").and_then(Value::as_array).filter(|v|!v.is_empty()&&v.len()<=1000).ok_or((StatusCode::BAD_REQUEST,"Select between 1 and 1,000 contacts.".into()))?;
    let mut db=db.lock().map_err(error)?;let tx=db.transaction().map_err(error)?;
    let mut seen=std::collections::HashSet::new();
    {
        let mut stmt=tx.prepare("SELECT data FROM records WHERE kind='guests'").map_err(error)?;
        let rows=stmt.query_map([],|r|r.get::<_,String>(0)).map_err(error)?;
        for row in rows {let g:Value=serde_json::from_str(&row.map_err(error)?).map_err(error)?;for (field,phone) in [("phone",true),("email",false)]{let key=contact_key(g[field].as_str().unwrap_or(""),phone);if !key.is_empty(){seen.insert(format!("{field}:{key}"));}}}
    }
    let mut imported=0;let mut skipped=0;
    for c in contacts {
        let name=c["name"].as_str().unwrap_or("").trim();
        if name.is_empty()||name.len()>300{return Err((StatusCode::BAD_REQUEST,"Each selected contact needs a name under 300 characters.".into()));}
        let phone=c["phone"].as_str().unwrap_or("");let email=c["email"].as_str().unwrap_or("");
        if phone.len()>100||email.len()>300{return Err((StatusCode::BAD_REQUEST,"A contact's phone or email is too long.".into()));}
        let keys=[("phone",contact_key(phone,true)),("email",contact_key(email,false))].into_iter().filter(|(_,v)|!v.is_empty()).map(|(f,v)|format!("{f}:{v}")).collect::<Vec<_>>();
        if keys.iter().any(|k|seen.contains(k)){skipped+=1;continue;}
        let id=uuid::Uuid::new_v4().to_string();
        let mut guest=json!({"id":id,"token":uuid::Uuid::new_v4().to_string(),"name":name,"phone":phone,"email":email,"rsvp":"Pending","contactImport":true});
        for field in ["category","tier","side","categoryReview","categoryReason"] {let value=c[field].as_str().unwrap_or("");if value.len()>500{return Err((StatusCode::BAD_REQUEST,"Contact detail is too long.".into()));}guest[field]=json!(value);}
        validate_and_rename(&tx,"guests",&id,&mut guest).map_err(|e|(StatusCode::BAD_REQUEST,e))?;
        tx.execute("INSERT INTO records(kind,id,data) VALUES('guests',?1,?2)",params![id,guest.to_string()]).map_err(error)?;
        seen.extend(keys);imported+=1;
    }
    tx.commit().map_err(error)?;Ok(Json(json!({"imported":imported,"skipped":skipped})))
}
#[cfg(test)]mod tests {
    use super::*;
    #[tokio::test]async fn reviewed_contact_import_is_atomic_and_skips_duplicates(){
        let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE records(kind TEXT,id TEXT,data TEXT,PRIMARY KEY(kind,id));").unwrap();initialize(&db).unwrap();
        let db=std::sync::Arc::new(std::sync::Mutex::new(db));
        let body=json!({"contacts":[{"name":"Mom","phone":"+91 1234","category":"Family","categoryReview":"suggested"},{"name":"Duplicate","phone":"+911234"},{"name":"Anita Work","email":"A@EXAMPLE.COM","category":"Colleagues","categoryReview":"confirmed"}]});
        let result=import_contacts(State(db.clone()),Json(body.clone())).await.unwrap().0;
        assert_eq!(result["imported"],2);assert_eq!(result["skipped"],1);
        let repeated=import_contacts(State(db.clone()),Json(body)).await.unwrap().0;assert_eq!(repeated["imported"],0);
        let bad=json!({"contacts":[{"name":"Would roll back","phone":"999"},{"name":""}]});
        assert!(import_contacts(State(db.clone()),Json(bad)).await.is_err());
        let count:i64=db.lock().unwrap().query_row("SELECT count(*) FROM records WHERE kind='guests'",[],|r|r.get(0)).unwrap();assert_eq!(count,2);
    }
    #[test]fn renaming_preserves_membership_and_deletion_checks_members(){
        let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE records(kind TEXT,id TEXT,data TEXT,PRIMARY KEY(kind,id));").unwrap();initialize(&db).unwrap();
        db.execute("INSERT INTO records VALUES('guests','g','{\"id\":\"g\",\"category\":\"Family\"}')",[]).unwrap();
        assert!(check_delete(&db,"categories","family").is_err());
        validate_and_rename(&db,"categories","family",&mut json!({"name":"Close family"})).unwrap();
        let category:String=db.query_row("SELECT json_extract(data,'$.category') FROM records WHERE id='g'",[],|r|r.get(0)).unwrap();assert_eq!(category,"Close family");
        assert!(validate_and_rename(&db,"categories","new",&mut json!({"name":"Friends"})).is_err());
    }
}
