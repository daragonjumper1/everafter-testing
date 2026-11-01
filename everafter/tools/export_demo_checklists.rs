use rusqlite::Connection;
use serde_json::{json,Value};
fn main()->Result<(),Box<dyn std::error::Error>> {
    let destination=std::env::args().nth(1).ok_or("Supply the output JSON path")?;
    let mut templates=json!({});
    for kind in everafter::checklists::KINDS {
        let db=Connection::open_in_memory()?;
        db.execute_batch("CREATE TABLE records(kind TEXT,id TEXT,data TEXT,PRIMARY KEY(kind,id));")?;
        everafter::checklists::apply(&db,kind,"2030-01-01")?;
        let mut stmt=db.prepare("SELECT data FROM records ORDER BY rowid")?;
        let rows=stmt.query_map([],|r|r.get::<_,String>(0))?;
        let mut tasks=Vec::new();for row in rows {tasks.push(serde_json::from_str::<Value>(&row?)?)}
        templates[*kind]=json!(tasks);
    }
    std::fs::write(destination,serde_json::to_string_pretty(&json!({"baseDate":"2030-01-01","templates":templates}))?)?;
    Ok(())
}
