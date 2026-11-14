//! Application state on the Rust side: current view (list of items), selection, grid layout.
use crate::{ai, db, scan, thumbs};
use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use rusqlite::{params_from_iter, types::Value, Connection};
use serde_json::{json, Value as J};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Mutex, MutexGuard, OnceLock};

#[derive(Clone)]
pub struct Item {
    pub id: i64,
    pub w: i32,
    pub h: i32,
    pub kind: i64,
    pub taken: i64,
    pub dur: f64,
    pub fav: bool,
}

pub struct State {
    pub conn: Connection,
    pub items: Vec<Item>,
    pub view: (String, String),
    pub sel: HashSet<i64>,
    pub anchor: Option<usize>,
    pub ordered: bool,
}

static ST: OnceLock<Mutex<State>> = OnceLock::new();
static AI_RES: Mutex<Option<HashMap<String, Vec<String>>>> = Mutex::new(None);
pub static AI_BUSY: AtomicBool = AtomicBool::new(false);

pub fn st() -> MutexGuard<'static, State> {
    ST.get().expect("stan niezainicjowany").lock().unwrap_or_else(|e| e.into_inner())
}

pub fn init() {
    let conn = db::open();
    {
        use rusqlite::functions::FunctionFlags;
        let _ = conn.create_scalar_function("ulower", 1, FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC, |ctx| {
            let s: String = ctx.get(0)?;
            Ok(s.to_lowercase())
        });
    }
    let src = db::resolve_source(&conn);
    *db::SOURCE.lock().unwrap_or_else(|e| e.into_inner()) = Some(src);
    let _ = ST.set(Mutex::new(State { conn, items: vec![], view: ("timeline".into(), String::new()), sel: HashSet::new(), anchor: None, ordered: false }));
    refresh();
}

pub static PL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn is_pl() -> bool {
    PL.load(std::sync::atomic::Ordering::Relaxed)
}
/// Text depending on the interface language.
pub fn tr(pl: &str, en: &str) -> String {
    (if is_pl() { pl } else { en }).to_string()
}
const MON_NOM_EN: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const MON_SHORT_EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const WD_LONG_EN: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
fn mon_nom(i: usize) -> &'static str { if is_pl() { MON_NOM[i] } else { MON_NOM_EN[i] } }
fn mon_gen(i: usize) -> &'static str { if is_pl() { MON_GEN[i] } else { MON_NOM_EN[i] } }
fn mon_short(i: usize) -> &'static str { if is_pl() { MON_SHORT[i] } else { MON_SHORT_EN[i] } }
fn wd_long(i: usize) -> &'static str { if is_pl() { WD_LONG[i] } else { WD_LONG_EN[i] } }
const MON_NOM: [&str; 12] = ["styczeń", "luty", "marzec", "kwiecień", "maj", "czerwiec", "lipiec", "sierpień", "wrzesień", "październik", "listopad", "grudzień"];
const MON_GEN: [&str; 12] = ["stycznia", "lutego", "marca", "kwietnia", "maja", "czerwca", "lipca", "sierpnia", "września", "października", "listopada", "grudnia"];
const MON_SHORT: [&str; 12] = ["sty", "lut", "mar", "kwi", "maj", "cze", "lip", "sie", "wrz", "paź", "lis", "gru"];
const WD_SHORT: [&str; 7] = ["pon.", "wt.", "śr.", "czw.", "pt.", "sob.", "niedz."];
const WD_LONG: [&str; 7] = ["poniedziałek", "wtorek", "środa", "czwartek", "piątek", "sobota", "niedziela"];

fn dt(ts: i64) -> NaiveDateTime {
    chrono::DateTime::from_timestamp(ts, 0).map(|d| d.naive_utc()).unwrap_or_default()
}

const COLS: &str = "p.id,p.w,p.h,p.kind,p.taken,p.dur,p.fav";

fn row_item(r: &rusqlite::Row) -> rusqlite::Result<Item> {
    Ok(Item { id: r.get(0)?, w: r.get(1)?, h: r.get(2)?, kind: r.get(3)?, taken: r.get(4)?, dur: r.get(5)?, fav: r.get::<_, i64>(6)? != 0 })
}

fn month_of(tok: &str) -> Option<usize> {
    (0..12).find(|&i| {
        let (a, b, e) = (MON_NOM[i], MON_GEN[i], MON_NOM_EN[i].to_lowercase());
        tok == a || tok == b || tok == e || (tok.chars().count() >= 4 && (a.starts_with(tok) || e.starts_with(tok)))
    })
}

fn search_sql(q: &str) -> (String, Vec<Value>) {
    let hay = "ulower(p.rel||' '||p.cam||' '||p.lens||' '||p.descr||' '||p.people||' '||IFNULL((SELECT group_concat(a.title,' ') FROM album_items ai JOIN albums a ON a.id=ai.album_id WHERE ai.rel=p.rel),''))";
    let mut conds = vec!["p.trashed=0".to_string()];
    let mut pars: Vec<Value> = vec![];
    for tok in q.to_lowercase().split_whitespace() {
        let like = format!("{hay} LIKE ?");
        let esc = format!("%{}%", tok.replace('%', "\\%").replace('_', "\\_"));
        let b = tok.as_bytes();
        let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
        if tok.len() == 4 && digits(tok) && (1990..=2100).contains(&tok.parse::<i32>().unwrap_or(0)) {
            conds.push(format!("(p.taken>0 AND strftime('%Y',p.taken,'unixepoch')=? OR {like})"));
            pars.push(Value::Text(tok.into()));
        } else if tok.len() == 7 && b[4] == b'-' && digits(&tok[..4]) && digits(&tok[5..]) {
            conds.push(format!("(p.taken>0 AND strftime('%Y-%m',p.taken,'unixepoch')=? OR {like})"));
            pars.push(Value::Text(tok.into()));
        } else if tok.len() == 10 && b[4] == b'-' && b[7] == b'-' {
            conds.push(format!("(p.taken>0 AND strftime('%Y-%m-%d',p.taken,'unixepoch')=? OR {like})"));
            pars.push(Value::Text(tok.into()));
        } else if let Some(m) = month_of(tok) {
            conds.push(format!("(p.taken>0 AND strftime('%m',p.taken,'unixepoch')=? OR {like})"));
            pars.push(Value::Text(format!("{:02}", m + 1)));
        } else {
            conds.push(like);
        }
        pars.push(Value::Text(esc));
    }
    (conds.join(" AND "), pars)
}

fn load_rels(conn: &Connection, rels: &[String]) -> Vec<Item> {
    let mut out = Vec::new();
    let Ok(mut st) = conn.prepare_cached(&format!("SELECT {COLS} FROM photos p WHERE p.rel=?1 AND p.trashed=0")) else { return out };
    for r in rels {
        if let Ok(it) = st.query_row([r], row_item) {
            out.push(it);
        }
    }
    out
}

/// Reload the current view from SQLite.
pub fn refresh() {
    let mut s = st();
    let (kind, arg) = s.view.clone();
    s.ordered = false;
    let order = "ORDER BY (p.taken=0), p.taken DESC, p.id DESC";
    let sql_where: Option<(String, Vec<Value>, String)> = match kind.as_str() {
        "timeline" => Some(("p.archived=0 AND p.trashed=0".into(), vec![], order.into())),
        "fav" => Some(("p.fav=1 AND p.trashed=0".into(), vec![], order.into())),
        "video" => Some(("p.kind=1 AND p.archived=0 AND p.trashed=0".into(), vec![], order.into())),
        "recent" => Some(("p.archived=0 AND p.trashed=0".into(), vec![], "ORDER BY p.added DESC, p.taken DESC, p.id DESC".into())),
        "archive" => Some(("p.archived=1 AND p.trashed=0".into(), vec![], order.into())),
        "trash" => Some(("p.trashed=2".into(), vec![], order.into())),
        "nodate" => Some(("p.taken=0 AND p.trashed=0 AND p.archived=0".into(), vec![], "ORDER BY p.rel".into())),
        "album" => Some(("p.trashed=0 AND p.rel IN (SELECT rel FROM album_items WHERE album_id=?)".into(), vec![Value::Integer(arg.parse().unwrap_or(0))], order.into())),
        "search" => {
            let (c, p) = search_sql(&arg);
            Some((c, p, order.into()))
        }
        _ => None,
    };
    let items: Vec<Item> = if let Some((wh, pars, ord)) = sql_where {
        let sql = format!("SELECT {COLS} FROM photos p WHERE {wh} {ord}");
        match s.conn.prepare(&sql) {
            Ok(mut st) => st.query_map(params_from_iter(pars.iter()), row_item).map(|x| x.flatten().collect()).unwrap_or_default(),
            Err(e) => {
                eprintln!("kula: SQL: {e}");
                vec![]
            }
        }
    } else if kind == "person" {
        let rels = ai::person_rels(arg.parse().unwrap_or(0));
        let mut v = load_rels(&s.conn, &rels);
        v.sort_by(|a, b| (a.taken == 0).cmp(&(b.taken == 0)).then(b.taken.cmp(&a.taken)));
        v
    } else if kind == "ai" {
        s.ordered = true;
        let rels = AI_RES.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|m| m.get(&arg).cloned()).unwrap_or_default();
        load_rels(&s.conn, &rels)
    } else {
        vec![]
    };
    let ids: HashSet<i64> = items.iter().map(|i| i.id).collect();
    s.sel.retain(|i| ids.contains(i));
    s.items = items;
    s.anchor = None;
}

/// Incremental removal of items from the current view (without re-running the SQL query):
/// keeps the order of the rest, clears the selection. Returns the number removed.
pub fn remove_ids(ids: &[i64]) -> usize {
    let mut s = st();
    let set: HashSet<i64> = ids.iter().copied().collect();
    let before = s.items.len();
    s.items.retain(|i| !set.contains(&i.id));
    s.sel.clear();
    s.anchor = None;
    before - s.items.len()
}

pub fn set_view(kind: &str, arg: &str) {
    {
        let mut s = st();
        s.view = (kind.to_string(), arg.to_string());
        s.sel.clear();
    }
    if kind == "ai" {
        let known = AI_RES.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|m| m.contains_key(arg)).unwrap_or(false);
        if !known {
            AI_BUSY.store(true, Relaxed);
            let q = arg.to_string();
            std::thread::spawn(move || {
                let rels = ai::semantic_search(&q);
                AI_RES.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(q, rels);
                AI_BUSY.store(false, Relaxed);
                scan::VIEW_VER.fetch_add(1, Relaxed);
            });
        }
    }
    refresh();
}

// ---------------------------------------------------------------- justified grid layout
fn day_title(ts: i64) -> String {
    let d = dt(ts);
    let cur = Local::now().year();
    let wd = d.weekday().num_days_from_monday() as usize;
    if d.year() == cur {
        if is_pl() { format!("{}, {} {}", wd_long(wd), d.day(), mon_short(d.month0() as usize)) } else { format!("{}, {} {}", wd_long(wd), mon_short(d.month0() as usize), d.day()) }
    } else {
        if is_pl() { format!("{}, {} {} {}", wd_long(wd), d.day(), mon_short(d.month0() as usize), d.year()) } else { format!("{}, {} {}, {}", wd_long(wd), mon_short(d.month0() as usize), d.day(), d.year()) }
    }
}

const GAP: f64 = 4.0;
const HDR_H: f64 = 50.0;

pub fn layout(width: f64, rowh: f64) -> String {
    let s = st();
    let avail = width.max(240.0);
    let monthly = rowh <= 125.0;
    let mut rows: Vec<J> = Vec::new();
    let mut marks: Vec<(f64, String)> = Vec::new();
    let mut y = 0.0f64;
    let n = s.items.len();
    let mut i = 0;
    let key = |it: &Item| -> i64 {
        if s.ordered {
            0
        } else if it.taken == 0 {
            i64::MIN
        } else if monthly {
            let d = dt(it.taken);
            d.year() as i64 * 12 + d.month0() as i64
        } else {
            it.taken.div_euclid(86400)
        }
    };
    while i < n {
        let k = key(&s.items[i]);
        let mut j = i;
        while j < n && key(&s.items[j]) == k {
            j += 1;
        }
        if !s.ordered {
            let t = s.items[i].taken;
            let (title, mark) = if t == 0 {
                { let n = tr("Bez daty", "No date"); (n.clone(), n) }
            } else {
                let d = dt(t);
                let m = format!("{} {}", mon_short(d.month0() as usize), d.year());
                (if monthly { format!("{} {}", mon_nom(d.month0() as usize), d.year()) } else { day_title(t) }, m)
            };
            if marks.last().map(|m| m.1 != mark).unwrap_or(true) {
                marks.push((y, mark));
            }
            rows.push(json!({"t":"h","h":HDR_H,"title":title,"a":i,"b":j-1}));
            y += HDR_H;
        }
        let mut row: Vec<usize> = vec![];
        let mut sum = 0.0;
        let flush = |row: &mut Vec<usize>, sum: &mut f64, last: bool, rows: &mut Vec<J>, y: &mut f64| {
            if row.is_empty() {
                return;
            }
            let gaps = GAP * (row.len() as f64 - 1.0);
            let natural = *sum * rowh + gaps;
            // full row: fit to width; last row of the day: stretch only if almost full
            let stretch = !last || natural >= avail * 0.62;
            let h = if stretch { ((avail - gaps) / *sum).min(rowh * 1.45) } else { rowh };
            let full = stretch && (avail - gaps) / *sum <= rowh * 1.45;
            let mut ws: Vec<f64> = row
                .iter()
                .map(|&ix| {
                    let it = &s.items[ix];
                    let asp = (it.w.max(1) as f64 / it.h.max(1) as f64).clamp(0.5, 3.0);
                    (asp * h).floor()
                })
                .collect();
            if full {
                let tot: f64 = ws.iter().sum::<f64>() + gaps;
                if let Some(l) = ws.last_mut() {
                    *l += (avail - tot).max(0.0);
                }
            }
            let items: Vec<J> = row.iter().zip(ws.iter()).map(|(&ix, w)| {
                let it = &s.items[ix];
                json!([it.id, w, ix, it.kind, it.dur, it.fav as i32])
            }).collect();
            rows.push(json!({"t":"r","h":h.floor()+GAP,"it":items}));
            *y += h.floor() + GAP;
            row.clear();
            *sum = 0.0;
        };
        for ix in i..j {
            let it = &s.items[ix];
            let asp = (it.w.max(1) as f64 / it.h.max(1) as f64).clamp(0.5, 3.0);
            row.push(ix);
            sum += asp;
            if sum * rowh + GAP * (row.len() as f64 - 1.0) >= avail {
                flush(&mut row, &mut sum, false, &mut rows, &mut y);
            }
        }
        flush(&mut row, &mut sum, true, &mut rows, &mut y);
        y += 8.0;
        rows.push(json!({"t":"s","h":8}));
        i = j;
    }
    json!({"rows":rows,"total":y,"marks":marks.iter().map(|m| json!([m.0, m.1])).collect::<Vec<_>>(),"count":n}).to_string()
}

// ---------------------------------------------------------------- thumbnails and info
pub fn thumb_url(id: i64) -> String {
    let s = st();
    let r = s.conn.prepare_cached("SELECT rel,mtime,size,thumb FROM photos WHERE id=?1").and_then(|mut q| q.query_row([id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))));
    drop(s);
    let Ok((rel, mt, sz, th)) = r else { return "!".into() };
    if let Some(p) = thumbs::thumb_exists(&rel, mt, sz) {
        return db::file_url(&p);
    }
    if th == 2 {
        return "!".into();
    }
    thumbs::want(id);
    String::new()
}

pub fn file_url_of(id: i64) -> String {
    let s = st();
    let rel: Option<String> = s.conn.query_row("SELECT rel FROM photos WHERE id=?1", [id], |r| r.get(0)).ok();
    rel.map(|r| db::file_url(&db::source().join(r))).unwrap_or_default()
}

fn full_date(ts: i64) -> String {
    let d = dt(ts);
    let (w, m) = (wd_long(d.weekday().num_days_from_monday() as usize), d.month0() as usize);
    if is_pl() { format!("{}, {} {} {}, {:02}:{:02}", w, d.day(), mon_gen(m), d.year(), d.hour(), d.minute()) } else { format!("{}, {} {}, {}, {:02}:{:02}", w, mon_nom(m), d.day(), d.year(), d.hour(), d.minute()) }
}

pub fn item_json(id: i64) -> String {
    let s = st();
    let r = s.conn.query_row(
        "SELECT rel,folder,name,size,mtime,kind,taken,w,h,dur,cam,lens,fnum,exposure,iso,focal,lat,lon,descr,people,fav,archived,trashed,date_source,date_quality FROM photos WHERE id=?1",
        [id],
        |r| {
            Ok((
                r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?, r.get::<_, i64>(4)?, r.get::<_, i64>(5)?, r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?, r.get::<_, i64>(8)?, r.get::<_, f64>(9)?, r.get::<_, String>(10)?, r.get::<_, String>(11)?, r.get::<_, f64>(12)?, r.get::<_, String>(13)?,
                r.get::<_, i64>(14)?, r.get::<_, f64>(15)?, r.get::<_, Option<f64>>(16)?, r.get::<_, Option<f64>>(17)?, r.get::<_, String>(18)?, r.get::<_, String>(19)?,
                r.get::<_, i64>(20)?, r.get::<_, i64>(21)?, r.get::<_, i64>(22)?, r.get::<_, String>(23)?, r.get::<_, String>(24)?,
            ))
        },
    );
    let Ok(t) = r else { return "{}".into() };
    let albums: Vec<String> = s
        .conn
        .prepare("SELECT a.title FROM album_items i JOIN albums a ON a.id=i.album_id WHERE i.rel=?1 ORDER BY a.title")
        .and_then(|mut q| q.query_map([&t.0], |r| r.get::<_, String>(0)).map(|x| x.flatten().collect()))
        .unwrap_or_default();
    json!({
        "id":id,"rel":t.0,"folder":t.1,"name":t.2,"size":t.3,"kind":t.5,"taken":t.6,
        "date": if t.6==0 {tr("Bez daty", "No date")} else {full_date(t.6)},
        "w":t.7,"h":t.8,"mpix":((t.7 as f64*t.8 as f64)/1e5).round()/10.0,"dur":t.9,"cam":t.10,"lens":t.11,"fnum":t.12,"exposure":t.13,"iso":t.14,"focal":t.15,
        "lat":t.16,"lon":t.17,"descr":t.18,"people":t.19,"fav":t.20!=0,"archived":t.21!=0,"trashed":t.22!=0,
        "date_source":t.23,"date_quality":match t.24.as_str() { "przybliżona" => "approx", "pewna" => "exact", _ => "none" },"albums":albums,
        "abs": db::source().join(&t.0).to_string_lossy(),
    })
    .to_string()
}

// ---------------------------------------------------------------- selection and flags
fn targets(s: &State, id: i64) -> Vec<i64> {
    if id >= 0 {
        vec![id]
    } else {
        s.items.iter().filter(|i| s.sel.contains(&i.id)).map(|i| i.id).collect()
    }
}

pub fn toggle_sel(id: i64) -> usize {
    let mut s = st();
    if !s.sel.remove(&id) {
        s.sel.insert(id);
    }
    s.anchor = s.items.iter().position(|i| i.id == id);
    s.sel.len()
}

pub fn range_sel(idx: usize) -> usize {
    let mut s = st();
    match s.anchor {
        Some(a) if a < s.items.len() && idx < s.items.len() => {
            let (lo, hi) = (a.min(idx), a.max(idx));
            let ids: Vec<i64> = s.items[lo..=hi].iter().map(|i| i.id).collect();
            s.sel.extend(ids);
        }
        _ => {
            if let Some(it) = s.items.get(idx).map(|i| i.id) {
                s.sel.insert(it);
            }
        }
    }
    s.anchor = Some(idx);
    s.sel.len()
}

pub fn sel_group(a: usize, b: usize, on: bool) -> usize {
    let mut s = st();
    if a < s.items.len() {
        let b = b.min(s.items.len() - 1);
        let ids: Vec<i64> = s.items[a..=b].iter().map(|i| i.id).collect();
        for i in ids {
            if on {
                s.sel.insert(i);
            } else {
                s.sel.remove(&i);
            }
        }
    }
    s.sel.len()
}

pub fn group_selected(a: usize, b: usize) -> bool {
    let s = st();
    a < s.items.len() && s.items[a..=b.min(s.items.len() - 1)].iter().all(|i| s.sel.contains(&i.id))
}

pub fn sel_all() -> usize {
    let mut s = st();
    let ids: Vec<i64> = s.items.iter().map(|i| i.id).collect();
    s.sel.extend(ids);
    s.sel.len()
}

pub fn clear_sel() {
    st().sel.clear();
}

pub fn is_selected(id: i64) -> bool {
    st().sel.contains(&id)
}

pub fn sel_count() -> usize {
    st().sel.len()
}

/// Sets a flag (fav/archived/trashed) in SQLite. val: 1/0, -1 = toggle based on the first. Returns (changed, whether layout needs recalculation).
pub fn mark(kind: &str, id: i64, val: i32) -> (usize, bool) {
    let col = match kind {
        "fav" => "fav",
        "archived" => "archived",
        "trashed" => "trashed",
        _ => return (0, false),
    };
    let (ids, newv) = {
        let s = st();
        let ids = targets(&s, id);
        if ids.is_empty() {
            return (0, false);
        }
        let newv = if val >= 0 {
            val as i64
        } else {
            let cur: i64 = s.conn.query_row(&format!("SELECT {col} FROM photos WHERE id=?1"), [ids[0]], |r| r.get(0)).unwrap_or(0);
            1 - cur
        };
        let _ = s.conn.execute_batch("BEGIN");
        for i in &ids {
            let _ = s.conn.execute(&format!("UPDATE photos SET {col}=?1 WHERE id=?2"), rusqlite::params![newv, i]);
        }
        let _ = s.conn.execute_batch("COMMIT");
        (ids, newv)
    };
    let kind = st().view.0.clone();
    let relayout = if col == "fav" {
        if kind == "fav" && newv == 0 {
            remove_ids(&ids);
        } else {
            let mut s = st();
            for it in s.items.iter_mut() {
                if ids.contains(&it.id) {
                    it.fav = newv != 0;
                }
            }
        }
        true
    } else if col == "archived" && matches!(kind.as_str(), "archive" | "timeline" | "video" | "recent" | "nodate") {
        // these views filter by archived: the item simply disappears from the list
        let leaves = if kind == "archive" { newv == 0 } else { newv == 1 };
        if leaves {
            remove_ids(&ids);
        }
        true
    } else {
        refresh();
        true
    };
    (ids.len(), relayout)
}

pub fn set_desc(id: i64, text: &str) {
    let s = st();
    let _ = s.conn.execute("UPDATE photos SET descr=?1 WHERE id=?2", rusqlite::params![text, id]);
}

// ---------------------------------------------------------------- albums
pub fn albums_json() -> String {
    let s = st();
    let sql = "SELECT a.id,a.title,a.kind,
        (SELECT COUNT(*) FROM album_items i JOIN photos p ON p.rel=i.rel WHERE i.album_id=a.id AND p.trashed=0) AS cnt,
        (SELECT p.id FROM photos p WHERE p.rel=COALESCE(a.cover_rel,(SELECT rel FROM album_items WHERE album_id=a.id LIMIT 1))),
        COALESCE((SELECT MAX(p.taken) FROM album_items i JOIN photos p ON p.rel=i.rel WHERE i.album_id=a.id),0)
        FROM albums a ORDER BY 6 DESC, a.title";
    let Ok(mut q) = s.conn.prepare(sql) else { return "[]".into() };
    let v: Vec<J> = q
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?, r.get::<_, Option<i64>>(4)?, r.get::<_, i64>(5)?)))
        .map(|x| x.flatten().filter(|t| t.3 > 0 || t.2 == "local").map(|t| json!({"id":t.0,"title":fix_title(&t.1),"kind":t.2,"count":t.3,"cover":t.4.unwrap_or(-1),"newest":t.5})).collect())
        .unwrap_or_default();
    J::Array(v).to_string()
}

pub fn album_create(title: &str) -> i64 {
    let s = st();
    let t = if title.trim().is_empty() { &tr("Album bez nazwy", "Untitled album") } else { title.trim() };
    let _ = s.conn.execute("INSERT INTO albums(title,kind,created) VALUES(?1,'local',?2)", rusqlite::params![t, db::now()]);
    s.conn.last_insert_rowid()
}

pub fn album_add(aid: i64, id: i64) -> usize {
    let s = st();
    let ids = targets(&s, id);
    let _ = s.conn.execute_batch("BEGIN");
    let mut n = 0;
    for i in &ids {
        if let Ok(rel) = s.conn.query_row("SELECT rel FROM photos WHERE id=?1", [i], |r| r.get::<_, String>(0)) {
            n += s.conn.execute("INSERT OR IGNORE INTO album_items(album_id,rel) VALUES(?1,?2)", rusqlite::params![aid, rel]).unwrap_or(0);
        }
    }
    let _ = s.conn.execute_batch("COMMIT");
    n
}

pub fn album_remove(aid: i64, id: i64) -> usize {
    let n = {
        let s = st();
        let ids = targets(&s, id);
        let mut n = 0;
        for i in &ids {
            if let Ok(rel) = s.conn.query_row("SELECT rel FROM photos WHERE id=?1", [i], |r| r.get::<_, String>(0)) {
                n += s.conn.execute("DELETE FROM album_items WHERE album_id=?1 AND rel=?2", rusqlite::params![aid, rel]).unwrap_or(0);
            }
        }
        n
    };
    refresh();
    n
}

pub fn album_rename(aid: i64, title: &str) {
    let s = st();
    if !title.trim().is_empty() {
        let _ = s.conn.execute("UPDATE albums SET title=?1 WHERE id=?2 AND kind='local'", rusqlite::params![title.trim(), aid]);
    }
}

pub fn album_delete(aid: i64) {
    let s = st();
    let _ = s.conn.execute("DELETE FROM album_items WHERE album_id=?1 AND (SELECT kind FROM albums WHERE id=?1)='local'", [aid]);
    let _ = s.conn.execute("DELETE FROM albums WHERE id=?1 AND kind='local'", [aid]);
}

pub fn album_title(aid: i64) -> String {
    st().conn.query_row("SELECT title FROM albums WHERE id=?1", [aid], |r| r.get(0)).unwrap_or_default()
}

pub fn album_kind(aid: i64) -> String {
    st().conn.query_row("SELECT kind FROM albums WHERE id=?1", [aid], |r| r.get(0)).unwrap_or_default()
}

pub fn persons_json() -> String {
    let ps = ai::persons();
    let s = st();
    let v: Vec<J> = ps
        .iter()
        .filter_map(|p| {
            let (cid, w, h): (i64, i64, i64) = s.conn.query_row("SELECT id,w,h FROM photos WHERE rel=?1", [&p.cover_rel], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).ok()?;
            Some(json!({"id":p.id,"name":p.name,"count":p.count,"cover":cid,"w":w,"h":h,"box":p.bx,"face":ai::face_url(p.id, &p.cover_rel, p.bx)}))
        })
        .collect();
    J::Array(v).to_string()
}

pub fn counts_json() -> String {
    let s = st();
    let q = |sql: &str| -> i64 { s.conn.query_row(sql, [], |r| r.get(0)).unwrap_or(0) };
    json!({
        "all": q("SELECT COUNT(*) FROM photos"),
        "timeline": q("SELECT COUNT(*) FROM photos WHERE archived=0 AND trashed=0"),
        "fav": q("SELECT COUNT(*) FROM photos WHERE fav=1 AND trashed=0"),
        "video": q("SELECT COUNT(*) FROM photos WHERE kind=1 AND archived=0 AND trashed=0"),
        "archive": q("SELECT COUNT(*) FROM photos WHERE archived=1 AND trashed=0"),
        "trash": q("SELECT COUNT(*) FROM photos WHERE trashed=2"),
        "nodate": q("SELECT COUNT(*) FROM photos WHERE taken=0 AND trashed=0 AND archived=0"),
    })
    .to_string()
}

pub fn index_of(id: i64) -> i64 {
    st().items.iter().position(|i| i.id == id).map(|p| p as i64).unwrap_or(-1)
}

pub fn id_at(idx: i64) -> i64 {
    let s = st();
    if idx < 0 {
        return -1;
    }
    s.items.get(idx as usize).map(|i| i.id).unwrap_or(-1)
}

pub fn count() -> usize {
    st().items.len()
}

/// "Bez nazwy(1)" → "Bez nazwy (1)"
pub fn fix_title(t: &str) -> String {
    if let Some(i) = t.rfind('(') {
        if i > 0 && !t[..i].ends_with(' ') && t.ends_with(')') {
            return format!("{} {}", &t[..i], &t[i..]);
        }
    }
    t.to_string()
}
