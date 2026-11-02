use rusqlite::{params, Connection};
use serde_json::json;

pub const KINDS: &[&str] = &["Choose later", "Civil / secular", "Hindu", "Muslim / Nikah", "Christian", "Sikh / Anand Karaj", "Jewish", "Buddhist", "Jain", "Interfaith / multicultural", "Custom ceremony"];

// These are editable planning prompts, not mandatory religious or legal requirements.
pub fn apply(db: &Connection, kind: &str, date: &str) -> rusqlite::Result<usize> {
    if kind == "Choose later" || kind.is_empty() { return Ok(0); }
    let common = [
        ("budget", "Agree on your wedding budget and priorities", 180),
        ("venue", "Shortlist and confirm ceremony and celebration venues", 150),
        ("guest-list", "Build your guest list and planning tiers", 120),
        ("officiant", "Confirm your officiant or ceremony leader", 120),
        ("requirements", "Check local marriage registration and document requirements", 120),
        ("vendors", "Compare and book catering, photography, and key vendors", 90),
        ("invitations", "Design invitations and collect mailing addresses", 75),
        ("rsvp", "Send invitations and set an RSVP deadline", 60),
        ("logistics", "Arrange guest accommodation and transport", 45),
        ("preferences", "Confirm meal preferences and accessibility needs", 30),
        ("run-sheet", "Create the day-of schedule and share key contacts", 14),
        ("final-confirmations", "Reconfirm vendors, guest counts, and remaining payments", 7),
    ];
    let mut count = 0;
    for (key, name, days) in common { count += insert(db, "common", key, name, days, date, "Suggested planning task. Adjust the date, owner, and notes to your plans. Registration rules vary; confirm with your local registrar.")?; }
    let extra: &[(&str, &str, i32)] = match kind {
        "Hindu" => &[("ceremony","Discuss the ceremony sequence and chosen rituals with your priest",90),("events","Choose any pre-wedding events you want to include",90),("materials","Confirm ceremony materials, attire, and venue arrangements with your priest",30),("participants","Brief the people taking part in your chosen rituals",14)],
        "Muslim / Nikah" => &[("ceremony","Discuss your preferred Nikah format with your officiant",90),("agreements","Discuss ceremony agreements and documentation with your officiant",60),("participants","Confirm ceremony participants and their roles with your officiant",30),("celebrations","Plan any additional celebrations you choose to host",45)],
        "Christian" => &[("ceremony","Discuss your ceremony format with your minister or celebrant",90),("preparation","Ask about any preparation requested by your chosen church",90),("readings","Choose readings, music, and vows with your officiant",45),("rehearsal","Arrange a ceremony rehearsal if you want one",14)],
        "Sikh / Anand Karaj" => &[("ceremony","Discuss your Anand Karaj plans with your chosen gurdwara",120),("guidance","Confirm ceremony arrangements and guest guidance with the gurdwara",60),("hospitality","Coordinate any shared meal and hospitality arrangements",45),("participants","Share the agreed ceremony schedule with participants",14)],
        "Jewish" => &[("ceremony","Discuss your ceremony traditions with your rabbi or officiant",90),("documents","Discuss ceremony documents and participants with your officiant",60),("setting","Plan the ceremony setting and any ritual objects you choose",45),("readings","Confirm your selected blessings, readings, and music",30)],
        "Buddhist" => &[("ceremony","Choose your ceremony format with your monk or celebrant",90),("setting","Discuss any blessings, offerings, and ceremony setup you want",45),("participants","Confirm ceremony participants, readings, and guest guidance",30)],
        "Jain" => &[("ceremony","Discuss your chosen ceremony and customs with your officiant",90),("menu","Confirm your preferred menu and dietary arrangements",60),("materials","Confirm ceremony materials and the agreed sequence of events",30)],
        "Interfaith / multicultural" => &[("traditions","Discuss the traditions each partner wants to include",120),("leaders","Find officiants comfortable with your combined ceremony",90),("sequence","Agree on the ceremony sequence, languages, and readings",60),("guidance","Prepare a short guest guide to your chosen traditions",30),("family","Share the agreed plans with family and ceremony participants",30)],
        "Custom ceremony" => &[("vision","Write down your ceremony vision and meaningful customs",120),("leader","Choose a celebrant or someone to lead your ceremony",90),("script","Create your ceremony script, vows, and readings",45),("participants","Confirm participant roles and rehearse your chosen format",14)],
        _ => &[("ceremony","Plan your ceremony format, vows, and readings",60),("participants","Confirm ceremony participants and any rehearsal",30)],
    };
    for &(key, name, days) in extra { count += insert(db, kind, key, name, days, date, "Optional starter suggestion for your selected wedding style. Practices vary by community and personal preference. Confirm details with your chosen officiant or celebrant; edit or remove anything you do not need.")?; }
    Ok(count)
}
fn insert(db: &Connection, group: &str, key: &str, name: &str, days: i32, date: &str, notes: &str) -> rusqlite::Result<usize> {
    let id = format!("starter:{group}:{key}");
    let due: Option<String> = db.query_row("SELECT date(?1, ?2)", params![date, format!("-{days} days")], |r| r.get(0))?;
    let prerequisites:&[&str]=if group!="common"{&["officiant"]}else{match key{"venue"|"guest-list"=>&["budget"],"officiant"=>&["venue"],"vendors"=>&["venue","budget"],"invitations"=>&["guest-list","venue"],"rsvp"=>&["invitations"],"logistics"=>&["guest-list","venue"],"preferences"=>&["rsvp"],"run-sheet"=>&["vendors","officiant"],"final-confirmations"=>&["rsvp","vendors","logistics"],_=>&[]}};
    let dependency_ids:Vec<String>=prerequisites.iter().map(|p|format!("starter:common:{p}")).collect();
    let task = json!({"id":id,"name":name,"owner":"","due":due.unwrap_or_default(),"notes":notes,"done":false,"starterKind":group,"dependencyIds":dependency_ids});
    db.execute("INSERT OR IGNORE INTO records(kind,id,data) VALUES('tasks',?1,?2)",params![id,task.to_string()])
}
#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection { let db=Connection::open_in_memory().unwrap(); db.execute_batch("CREATE TABLE records(kind TEXT,id TEXT,data TEXT,PRIMARY KEY(kind,id));").unwrap();db }
    #[test]
    fn starters_are_dated_and_do_not_duplicate_or_overwrite_edits() {
        let db=db(); assert_eq!(apply(&db,"Hindu","2027-12-01").unwrap(),16);
        db.execute("UPDATE records SET data=json_set(data,'$.done',json('true'),'$.name','My edited task') WHERE id='starter:common:budget'",[]).unwrap();
        assert_eq!(apply(&db,"Hindu","2028-01-01").unwrap(),0);
        let (name,done,due):(String,bool,String)=db.query_row("SELECT json_extract(data,'$.name'),json_extract(data,'$.done'),json_extract(data,'$.due') FROM records WHERE id='starter:common:budget'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_eq!(name,"My edited task");assert!(done);assert_eq!(due,"2027-06-04");
    }
    #[test]
    fn switching_styles_preserves_previous_and_common_tasks() {
        let db=db();apply(&db,"Hindu","").unwrap();assert_eq!(apply(&db,"Christian","").unwrap(),4);
        let count:i64=db.query_row("SELECT count(*) FROM records",[],|r|r.get(0)).unwrap();assert_eq!(count,20);
        assert_eq!(apply(&db,"Choose later","").unwrap(),0);
    }
}
