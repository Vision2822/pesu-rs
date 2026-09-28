// PESU Scraper with Timetable + Attendance support
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use reqwest::{Method, RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const BASE_URL: &str = "https://www.pesuacademy.com/Academy";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub csrf_token: String,
    pub srn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Semester {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: String,
    pub code: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub name: String,
    pub srn: String,
    pub semester: String,
    pub course: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatingEntry {
    pub assessment: String,
    pub course_code: String,
    pub date: String,
    pub time: String,
    pub terminal: String,
    pub block: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodaySeating {
    pub course: String,
    pub date: String,
    pub block: String,
    pub terminal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatingInfo {
    pub notice_title: Option<String>,
    pub notice_body: Option<String>,
    pub today: Option<TodaySeating>,
    pub entries: Vec<SeatingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceRecord {
    pub code: String,
    pub name: String,
    pub attended: Option<i32>,
    pub total: Option<i32>,
    pub percentage: Option<f32>,
    pub raw_classes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceInfo {
    pub records: Vec<AttendanceRecord>,
}

// --- Timetable structs ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimetableSlot {
    pub ordered_by: i32,
    pub start: String,
    pub end: String,
    pub is_break: bool,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimetableEntry {
    pub code: String,
    pub title: String,
    pub faculty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimetableSlotData {
    pub ordered_by: i32,
    pub is_break: bool,
    pub entries: Vec<TimetableEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimetableDay {
    pub day: String,
    pub index: usize,
    pub slots: Vec<TimetableSlotData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimetableMeta {
    pub batch: String,
    pub class_name: String,
    pub department: String,
    pub section: String,
    pub room: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimetableInfo {
    pub slots: Vec<TimetableSlot>,
    pub days: Vec<TimetableDay>,
    pub meta: TimetableMeta,
}

#[derive(Debug, Deserialize)]
struct RawTemplateDetail {
    #[serde(rename = "startTime")]
    start_time: String,
    #[serde(rename = "endTime")]
    end_time: String,
    #[serde(rename = "orderedBy")]
    ordered_by: i32,
    #[serde(rename = "timeTableTemplateDetailsStatus")]
    status: i32,
}

pub struct PESUScraper {
    pub client: reqwest::Client,
    pub csrf_token: Option<String>,
    pub srn: Option<String>,
    pub authenticated_referer: Option<String>,
}

pub fn extract_csrf_token(html: &str) -> Option<String> {
    let doc = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("input[name='_csrf']").ok()?;
    doc.select(&selector)
        .next()
        .and_then(|el| el.value().attr("value"))
        .map(|s| s.to_string())
}

pub fn map_reqwest_error(err: reqwest::Error) -> String {
    if err.is_timeout() {
        "The request timed out. Try again.".to_string()
    } else if let Some(status) = err.status() {
        if status == StatusCode::FORBIDDEN {
            "Portal blocked the request (403 Forbidden). Try again later.".to_string()
        } else if status.is_server_error() {
            format!("PESU Academy server error ({status}). Please try again later.")
        } else {
            format!("Server returned HTTP error {status}.")
        }
    } else if err.is_connect() {
        "Could not reach PESU Academy. Check your connection.".to_string()
    } else {
        format!("Network error: {err}")
    }
}

pub fn check_response_status(status: StatusCode) -> Result<(), String> {
    if status == StatusCode::FORBIDDEN {
        Err("Portal blocked the request. Sign in again.".to_string())
    } else if status.is_server_error() {
        Err(format!(
            "PESU Academy server error ({status}). Please try again later."
        ))
    } else {
        Ok(())
    }
}

pub fn sanitize(name: &str, maxlen: usize) -> String {
    if name.trim().is_empty() {
        return "untitled".to_string();
    }
    let mut s = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            '\\' | '/' | '*' | '?' | ':' | '"' | '<' | '>' | '|' => s.push('-'),
            _ => s.push(c),
        }
    }
    let words: Vec<&str> = s.split_whitespace().collect();
    let collapsed = words.join(" ");
    let stripped = collapsed.trim_matches(|c: char| c == ' ' || c == '-' || c == '.' || c == '_');
    if stripped.is_empty() {
        return "untitled".to_string();
    }
    let truncated: String = stripped.chars().take(maxlen).collect();
    let final_val = truncated.trim().to_string();
    if final_val.is_empty() {
        "untitled".to_string()
    } else {
        final_val
    }
}

pub fn unescape_text(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    for _ in 0..2 {
        if (text.starts_with('"') && text.ends_with('"') && text.len() >= 2)
            || (text.starts_with('\'') && text.ends_with('\'') && text.len() >= 2)
        {
            if let Ok(serde_json::Value::String(unwrapped)) =
                serde_json::from_str::<serde_json::Value>(&text)
            {
                text = unwrapped;
            } else {
                text = text[1..text.len() - 1].to_string();
            }
        }
    }
    text.replace("\\\"", "\"")
        .replace("\\'", "'")
        .replace("\\/", "/")
}

fn extract_id_string(v: &serde_json::Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        Some(s.to_string())
    } else if let Some(n) = v.as_i64() {
        Some(n.to_string())
    } else if let Some(u) = v.as_u64() {
        Some(u.to_string())
    } else {
        None
    }
}

pub fn parse_semesters(raw_text: &str) -> Vec<Semester> {
    let text = unescape_text(raw_text);
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(arr) = val.as_array() {
            let mut semesters = Vec::new();
            for item in arr {
                if let Some(obj) = item.as_object() {
                    let sid_val = obj
                        .get("id")
                        .or_else(|| obj.get("value"))
                        .or_else(|| obj.get("sem"))
                        .or_else(|| obj.get("semesterId"));
                    if let Some(sid_raw) = sid_val.and_then(extract_id_string) {
                        let sid = sid_raw.trim().to_string();
                        if !sid.is_empty() && sid != "0" && !sid.eq_ignore_ascii_case("null") {
                            let sname = obj
                                .get("name")
                                .or_else(|| obj.get("text"))
                                .or_else(|| obj.get("semester"))
                                .or_else(|| obj.get("semesterName"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|| format!("Semester {sid}"));
                            semesters.push(Semester { id: sid, name: sname });
                        }
                    }
                } else if let Some(sid_raw) = extract_id_string(item) {
                    let sid = sid_raw.trim().to_string();
                    if !sid.is_empty() && sid != "0" && !sid.eq_ignore_ascii_case("null") {
                        semesters.push(Semester {
                            id: sid.clone(),
                            name: format!("Semester {sid}"),
                        });
                    }
                }
            }
            if !semesters.is_empty() {
                return semesters;
            }
        }
        if let Some(obj) = val.as_object() {
            let mut semesters = Vec::new();
            for (k, v) in obj {
                let k_trimmed = k.trim().to_string();
                if !k_trimmed.is_empty()
                    && k_trimmed != "0"
                    && !k_trimmed.eq_ignore_ascii_case("null")
                {
                    let name = match v {
                        serde_json::Value::String(s) => s.trim().to_string(),
                        serde_json::Value::Number(n) => n.to_string(),
                        _ => format!("Semester {k_trimmed}"),
                    };
                    semesters.push(Semester {
                        id: k_trimmed,
                        name,
                    });
                }
            }
            if !semesters.is_empty() {
                return semesters;
            }
        }
    }
    if let Ok(re) = regex_lite::Regex::new(
        r#"(?i)<option\s+[^>]*?value=["']?([^"'>\s]+)["']?[^>]*>([^<]+)</option>"#,
    ) {
        let mut semesters = Vec::new();
        for cap in re.captures_iter(&text) {
            if let (Some(val_m), Some(name_m)) = (cap.get(1), cap.get(2)) {
                let val = val_m.as_str().trim();
                let name = name_m.as_str().trim();
                if !val.is_empty()
                    && val != "0"
                    && !val.eq_ignore_ascii_case("null")
                    && !name.to_lowercase().contains("select")
                {
                    semesters.push(Semester {
                        id: val.to_string(),
                        name: name.to_string(),
                    });
                }
            }
        }
        if !semesters.is_empty() {
            return semesters;
        }
    }
    Vec::new()
}

pub fn parse_courses_html(html: &str) -> Vec<Course> {
    let doc = scraper::Html::parse_document(html);
    let tr_selector = match scraper::Selector::parse("tr[onclick]") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let td_selector = match scraper::Selector::parse("td") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let re = match regex_lite::Regex::new(r"clickOnCourseContent\('(\d+)'") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut courses_map: BTreeMap<String, Course> = BTreeMap::new();
    let mut ordered_ids: Vec<String> = Vec::new();
    for tr in doc.select(&tr_selector) {
        if let Some(id_attr) = tr.value().attr("id") {
            if id_attr.starts_with("subrow") {
                continue;
            }
        }
        let onclick = tr.value().attr("onclick").unwrap_or("");
        if let Some(caps) = re.captures(onclick) {
            if let Some(cid_m) = caps.get(1) {
                let cid = cid_m.as_str().to_string();
                let cols: Vec<_> = tr.select(&td_selector).collect();
                if cols.len() >= 2 {
                    let code_raw: String = cols[0].text().collect::<Vec<_>>().join(" ");
                    let title_raw: String = cols[1].text().collect::<Vec<_>>().join(" ");
                    let code = sanitize(&code_raw, 80);
                    let title = sanitize(&title_raw, 80);
                    if !courses_map.contains_key(&cid) {
                        ordered_ids.push(cid.clone());
                    }
                    courses_map.insert(
                        cid.clone(),
                        Course {
                            id: cid,
                            code,
                            title,
                        },
                    );
                }
            }
        }
    }
    ordered_ids
        .into_iter()
        .filter_map(|id| courses_map.remove(&id))
        .collect()
}

pub fn parse_user_profile_html(html: &str) -> Option<UserProfile> {
    let doc = scraper::Html::parse_document(html);
    let name_sel = scraper::Selector::parse(".userlogedin_info h4.info_header, .app-name-font").ok()?;
    let name = doc
        .select(&name_sel)
        .next()
        .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
        .filter(|s| !s.is_empty())?;
    let info_sel = scraper::Selector::parse(".userlogedin_info .info_text, .info_text").ok()?;
    let info_el = doc.select(&info_sel).next()?;
    let raw_inner = info_el.inner_html();
    let normalized = raw_inner
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<BR>", "\n")
        .replace("<BR/>", "\n")
        .replace("<BR />", "\n");
    let re_tag = regex_lite::Regex::new(r"<[^>]*>").ok()?;
    let cleaned = re_tag.replace_all(&normalized, "");
    let lines: Vec<&str> = cleaned
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let re_srn = regex_lite::Regex::new(r"(?i)SRN\s*:\s*([A-Za-z0-9]+)").ok()?;
    let srn = re_srn
        .captures(&cleaned)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .or_else(|| {
            lines.iter().find_map(|l| {
                if l.to_uppercase().starts_with("SRN") {
                    l.split(':').nth(1).map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
        })?;
    let re_sem = regex_lite::Regex::new(r"Sem-\d+").ok()?;
    let semester = re_sem
        .captures(&cleaned)
        .and_then(|c| c.get(0))
        .map(|m| m.as_str().to_string())
        .or_else(|| {
            lines
                .iter()
                .find(|l| l.starts_with("Sem-"))
                .map(|l| l.to_string())
        })?;
    let sem_idx = lines
        .iter()
        .position(|l| re_sem.is_match(l) || l.starts_with("Sem-"))?;
    let course = lines
        .get(sem_idx + 1)
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())?;
    Some(UserProfile {
        name,
        srn,
        semester,
        course,
    })
}

pub fn parse_seating_html(html: &str) -> SeatingInfo {
    let doc = scraper::Html::parse_document(html);
    let mut notice_title = None;
    let mut notice_body = None;
    if let Ok(notice_sel) = scraper::Selector::parse(".highlight_snackbar1") {
        if let Some(notice_el) = doc.select(&notice_sel).next() {
            if let Ok(h3_sel) = scraper::Selector::parse("h3") {
                if let Some(h3_el) = notice_el.select(&h3_sel).next() {
                    let title = h3_el.text().collect::<Vec<_>>().join(" ");
                    let trimmed = title.trim();
                    if !trimmed.is_empty() {
                        notice_title = Some(trimmed.to_string());
                    }
                }
            }
            let inner_html = notice_el.inner_html();
            if let Ok(re_h3) = regex_lite::Regex::new(r"(?s)<h3[^>]*>.*?</h3>") {
                let replaced = re_h3.replace(&inner_html, "");
                let trimmed = replaced.trim();
                if !trimmed.is_empty() {
                    notice_body = Some(trimmed.to_string());
                }
            } else if let Some(end) = inner_html.find("</h3>") {
                let trimmed = inner_html[end + 5..].trim();
                if !trimmed.is_empty() {
                    notice_body = Some(trimmed.to_string());
                }
            }
        }
    }
    let mut today = None;
    if let Ok(bar_sel) = scraper::Selector::parse(".dashboard-info-bar") {
        if let Some(bar_el) = doc.select(&bar_sel).next() {
            let mut course = String::new();
            let mut date = String::new();
            let mut block = String::new();
            let mut terminal = String::new();
            if let Ok(div_sel) = scraper::Selector::parse(".dashboard-info-bar > div") {
                for div in bar_el.select(&div_sel) {
                    let mut label = String::new();
                    let mut val_parts = Vec::new();
                    for child in div.children() {
                        if let Some(el) = child.value().as_element() {
                            if el.name() == "h6" {
                                if let Some(ref_el) = scraper::ElementRef::wrap(child) {
                                    label = ref_el.text().collect::<Vec<_>>().join(" ").trim().to_lowercase();
                                }
                            }
                        } else if let Some(t) = child.value().as_text() {
                            let text_chunk = t.trim();
                            if !text_chunk.is_empty() {
                                val_parts.push(text_chunk);
                            }
                        }
                    }
                    let val = val_parts.join(" ");
                    if label.contains("course") {
                        course = val;
                    } else if label.contains("date") {
                        date = val;
                    } else if label.contains("block") {
                        block = val;
                    } else if label.contains("terminal") {
                        terminal = val;
                    }
                }
                if !course.is_empty() || !date.is_empty() || !block.is_empty() || !terminal.is_empty() {
                    today = Some(TodaySeating {
                        course,
                        date,
                        block,
                        terminal,
                    });
                }
            }
        }
    }
    let mut entries = Vec::new();
    if let Ok(row_sel) = scraper::Selector::parse("#seatinginfo tbody tr") {
        let td_sel = scraper::Selector::parse("td").ok();
        for tr in doc.select(&row_sel) {
            if let Some(ref sel) = td_sel {
                let cols: Vec<_> = tr
                    .select(sel)
                    .map(|c| c.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .collect();
                if cols.len() >= 6 {
                    entries.push(SeatingEntry {
                        assessment: cols[0].clone(),
                        course_code: cols[1].clone(),
                        date: cols[2].clone(),
                        time: cols[3].clone(),
                        terminal: cols[4].clone(),
                        block: cols[5].clone(),
                    });
                }
            }
        }
    }
    SeatingInfo {
        notice_title,
        notice_body,
        today,
        entries,
    }
}

// --- Attendance parsing ---

pub fn parse_attendance_html(html: &str) -> AttendanceInfo {
    let doc = scraper::Html::parse_document(html);
    let mut records = Vec::new();

    // selectors ordered by specificity
    let selectors = [
        "#subjetInfo tr",
        "table.box-shadow tbody tr",
        "table.table tbody tr",
        "tbody tr",
    ];

    for sel_str in selectors {
        if let Ok(sel) = scraper::Selector::parse(sel_str) {
            let mut found = false;
            for tr in doc.select(&sel) {
                // skip header rows with th
                if let Ok(th_sel) = scraper::Selector::parse("th") {
                    if tr.select(&th_sel).next().is_some() {
                        continue;
                    }
                }
                let td_sel = match scraper::Selector::parse("td") {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let cols: Vec<String> = tr
                    .select(&td_sel)
                    .map(|c| c.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .collect();
                if cols.len() < 2 {
                    continue;
                }
                let code = cols.get(0).cloned().unwrap_or_default().trim().to_string();
                if code.is_empty() || code.to_lowercase().contains("course code") {
                    continue;
                }
                let name = cols.get(1).cloned().unwrap_or_default().trim().to_string();
                let raw_classes = cols.get(2).cloned().unwrap_or_default().trim().to_string();
                let perc_raw = cols.get(3).cloned().unwrap_or_default().trim().to_string();

                let mut attended: Option<i32> = None;
                let mut total: Option<i32> = None;
                if raw_classes.contains('/') {
                    let parts: Vec<&str> = raw_classes.split('/').collect();
                    if parts.len() == 2 {
                        if let Ok(a) = parts[0].trim().parse::<i32>() {
                            attended = Some(a);
                        }
                        if let Ok(t) = parts[1].trim().parse::<i32>() {
                            total = Some(t);
                        }
                    }
                }

                let mut percentage: Option<f32> = None;
                let pr = perc_raw.to_lowercase();
                if pr != "na" && !pr.is_empty() && pr != "-" {
                    let cleaned = pr.replace('%', "").trim().to_string();
                    if let Ok(p) = cleaned.parse::<f32>() {
                        percentage = Some(p);
                    }
                }

                if percentage.is_none() {
                    if let (Some(a), Some(t)) = (attended, total) {
                        if t > 0 {
                            percentage = Some((a as f32 / t as f32) * 100.0);
                        }
                    }
                }

                found = true;
                records.push(AttendanceRecord {
                    code: sanitize(&code, 40),
                    name: sanitize(&name, 120),
                    attended,
                    total,
                    percentage,
                    raw_classes: raw_classes.clone(),
                });
            }
            if found && !records.is_empty() {
                break;
            }
        }
    }

    // deduplicate by code
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for r in records {
        if seen.insert(r.code.clone()) {
            deduped.push(r);
        }
    }

    AttendanceInfo { records: deduped }
}

// --- Timetable parsing ---

fn short_time(raw: &str) -> String {
    let parts: Vec<&str> = raw.trim().split_whitespace().collect();
    if parts.len() >= 2 {
        let hm = parts[0].split(':').take(2).collect::<Vec<_>>().join(":");
        format!("{} {}", hm, parts[1])
    } else {
        raw.trim().to_string()
    }
}

fn extract_meta_field(html: &str, label: &str) -> String {
    let pattern = format!(r#"(?i){}:</span>\s*([^<]+)"#, regex_lite::escape(label));
    if let Ok(re) = regex_lite::Regex::new(&pattern) {
        if let Some(cap) = re.captures(html) {
            if let Some(m) = cap.get(1) {
                return m.as_str().trim().to_string();
            }
        }
    }
    String::new()
}

pub fn parse_timetable_meta(html: &str) -> TimetableMeta {
    TimetableMeta {
        batch: extract_meta_field(html, "Batch"),
        class_name: extract_meta_field(html, "Class Name"),
        department: extract_meta_field(html, "Department"),
        section: extract_meta_field(html, "Section"),
        room: extract_meta_field(html, "Room"),
    }
}

pub fn parse_timetable_html(html: &str) -> Result<TimetableInfo, String> {
    let meta = parse_timetable_meta(html);

    let re_template = regex_lite::Regex::new(r"(?s)var timeTableTemplateDetailsJson\s*=\s*(\[.*?\]);")
        .map_err(|e| format!("regex error: {e}"))?;
    let re_days = regex_lite::Regex::new(r"(?s)var days\s*=\s*(\[.*?\]);")
        .map_err(|e| format!("regex error: {e}"))?;
    let re_tt = regex_lite::Regex::new(r"(?s)var timeTableJson\s*=\s*(\{.*\});\s*var sectionId")
        .map_err(|e| format!("regex error: {e}"))?;

    let template_json_str = re_template
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "Could not find timeTableTemplateDetailsJson".to_string())?;

    let days_json_str = re_days
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "Could not find days array".to_string())?;

    let tt_json_str = re_tt
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "Could not find timeTableJson".to_string())?;

    let raw_templates: Vec<RawTemplateDetail> = serde_json::from_str(&template_json_str)
        .map_err(|e| format!("Failed to parse template details: {e}"))?;
    let days: Vec<String> = serde_json::from_str(&days_json_str)
        .map_err(|e| format!("Failed to parse days: {e}"))?;
    let tt_map: BTreeMap<String, Vec<String>> = serde_json::from_str(&tt_json_str)
        .map_err(|e| format!("Failed to parse timetable map: {e}"))?;

    let mut templates_sorted = raw_templates;
    templates_sorted.sort_by_key(|t| t.ordered_by);
    let mut slots: Vec<TimetableSlot> = Vec::new();
    for t in &templates_sorted {
        let is_break = t.status == 1;
        let label = if is_break {
            "Break".to_string()
        } else {
            format!("{} - {}", short_time(&t.start_time), short_time(&t.end_time))
        };
        slots.push(TimetableSlot {
            ordered_by: t.ordered_by,
            start: short_time(&t.start_time),
            end: short_time(&t.end_time),
            is_break,
            label,
        });
    }

    let mut timetable_days: Vec<TimetableDay> = Vec::new();
    for (idx, day_name) in days.iter().enumerate() {
        let day_index = idx + 1;
        let mut slot_datas: Vec<TimetableSlotData> = Vec::new();
        for slot in &slots {
            if slot.is_break {
                slot_datas.push(TimetableSlotData {
                    ordered_by: slot.ordered_by,
                    is_break: true,
                    entries: vec![],
                });
                continue;
            }
            let prefix = format!("ttDivText_{}_{}_", day_index, slot.ordered_by);
            let mut entries: Vec<TimetableEntry> = Vec::new();
            for (key, vals) in &tt_map {
                if key.starts_with(&prefix) {
                    let mut code = String::new();
                    let mut title = String::new();
                    let mut faculties: Vec<String> = Vec::new();
                    for v in vals {
                        if v.starts_with("ttSubject_") {
                            if let Some(rest) = v.split("&&").nth(1) {
                                if let Some(dash_pos) = rest.find('-') {
                                    code = rest[..dash_pos].trim().to_string();
                                    title = rest[dash_pos + 1..].trim().to_string();
                                } else {
                                    code = rest.trim().to_string();
                                }
                            }
                        } else if v.starts_with("ttFaculty_") {
                            if let Some(rest) = v.split("&&").nth(1) {
                                let f = rest.trim().to_string();
                                if !f.is_empty() {
                                    faculties.push(f);
                                }
                            }
                        }
                    }
                    if !code.is_empty() || !title.is_empty() || !faculties.is_empty() {
                        entries.push(TimetableEntry {
                            code: code.clone(),
                            title: title.clone(),
                            faculty: faculties.join(", "),
                        });
                    }
                }
            }
            slot_datas.push(TimetableSlotData {
                ordered_by: slot.ordered_by,
                is_break: false,
                entries,
            });
        }
        timetable_days.push(TimetableDay {
            day: day_name.clone(),
            index: day_index,
            slots: slot_datas,
        });
    }

    Ok(TimetableInfo {
        slots,
        days: timetable_days,
        meta,
    })
}

impl PESUScraper {
    pub fn new() -> Result<Self, String> {
        let mut h = HeaderMap::new();
        h.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"));
        h.insert(ACCEPT, HeaderValue::from_static("*/*"));
        h.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.8"));
        h.insert("X-Requested-With", HeaderValue::from_static("XMLHttpRequest"));
        let client = reqwest::Client::builder().cookie_store(true).default_headers(h).timeout(Duration::from_secs(25)).build().map_err(|e| format!("HTTP client init failed: {e}"))?;
        Ok(Self { client, csrf_token: None, srn: None, authenticated_referer: None })
    }
    pub fn req(&self, method: Method, endpoint: &str) -> RequestBuilder {
        let url = format!("{BASE_URL}/{endpoint}");
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().to_string();
        let mut b = self.client.request(method, &url).query(&[("_", &ts)]);
        if let Some(ref csrf) = self.csrf_token { b = b.header("x-csrf-token", csrf); }
        if let Some(ref r) = self.authenticated_referer { b = b.header("Referer", r); }
        b
    }
    async fn refresh_csrf(&mut self) -> Result<(), String> {
        let url = format!("{BASE_URL}/s/studentProfilePESU");
        let r = self.client.get(&url).timeout(Duration::from_secs(15)).send().await.map_err(map_reqwest_error)?;
        if r.status().is_success() {
            let body = r.text().await.map_err(map_reqwest_error)?;
            if let Some(t) = extract_csrf_token(&body) { self.csrf_token = Some(t); }
        }
        Ok(())
    }

    pub async fn login(&mut self, srn: &str, password: &str) -> Result<LoginResponse, String> {
        let root_resp = self
            .client
            .get(format!("{BASE_URL}/"))
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(map_reqwest_error)?;
        check_response_status(root_resp.status())?;
        let root_html = root_resp.text().await.map_err(map_reqwest_error)?;
        let pre_csrf = extract_csrf_token(&root_html)
            .ok_or_else(|| "Portal returned an unexpected response. Try again.".to_string())?;
        let form_params = [
            ("j_username", srn),
            ("j_password", password),
            ("_csrf", pre_csrf.as_str()),
        ];
        let auth_resp = self
            .client
            .post(format!("{BASE_URL}/j_spring_security_check"))
            .form(&form_params)
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(map_reqwest_error)?;
        let final_url = auth_resp.url().clone();
        check_response_status(auth_resp.status())?;
        if final_url.as_str().contains("authfailed") {
            return Err("Incorrect SRN or password.".to_string());
        }
        let auth_html = auth_resp.text().await.map_err(map_reqwest_error)?;
        let post_csrf = extract_csrf_token(&auth_html).unwrap_or(pre_csrf);
        self.csrf_token = Some(post_csrf.clone());
        self.srn = Some(srn.to_string());
        self.authenticated_referer = Some(format!("{BASE_URL}/s/studentProfilePESU"));
        Ok(LoginResponse {
            success: true,
            csrf_token: post_csrf,
            srn: srn.to_string(),
        })
    }

    pub async fn get_semesters(&self) -> Result<Vec<Semester>, String> {
        let p1 = [
            ("menuId", "651"),
            ("url", "studentProfilePESUAdmin"),
            ("controllerMode", "6401"),
            ("actionType", "5"),
            ("id", "0"),
            ("selectedData", "0"),
        ];
        let _ = self
            .req(Method::GET, "s/studentProfilePESUAdmin")
            .query(&p1)
            .timeout(Duration::from_secs(15))
            .send()
            .await;
        let p2 = [
            ("menuId", "653"),
            ("url", "studentProfilePESUAdmin"),
            ("controllerMode", "6403"),
            ("actionType", "5"),
            ("id", "0"),
            ("selectedData", "0"),
        ];
        let _ = self
            .req(Method::GET, "s/studentProfilePESUAdmin")
            .query(&p2)
            .timeout(Duration::from_secs(15))
            .send()
            .await;
        let resp = self
            .req(Method::GET, "s/studentProfile/getStudentSemestersPESU")
            .timeout(Duration::from_secs(25))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    "Semester fetch timed out. Try again.".to_string()
                } else {
                    map_reqwest_error(e)
                }
            })?;
        check_response_status(resp.status())?;
        let body = resp.text().await.map_err(map_reqwest_error)?;
        if body.trim().is_empty() {
            return Err("Portal returned an unexpected response.".to_string());
        }
        let semesters = parse_semesters(&body);
        if semesters.is_empty() {
            return Err("No semesters found for this account.".to_string());
        }
        Ok(semesters)
    }

    pub async fn get_courses(&self, semester_id: &str) -> Result<Vec<Course>, String> {
        let form_params = [
            ("controllerMode", "6403"),
            ("actionType", "38"),
            ("id", semester_id),
            ("menuId", "653"),
        ];
        let resp = self
            .req(Method::POST, "s/studentProfilePESUAdmin")
            .form(&form_params)
            .timeout(Duration::from_secs(25))
            .send()
            .await
            .map_err(map_reqwest_error)?;
        check_response_status(resp.status())?;
        let text = resp.text().await.map_err(map_reqwest_error)?;
        let unescaped = unescape_text(&text);
        let courses = parse_courses_html(&unescaped);
        Ok(courses)
    }

    pub async fn get_user_profile(&self) -> Result<UserProfile, String> {
        let resp = self
            .req(Method::GET, "s/studentProfilePESU")
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .map_err(map_reqwest_error)?;
        check_response_status(resp.status())?;
        let html = resp.text().await.map_err(map_reqwest_error)?;
        parse_user_profile_html(&html).ok_or_else(|| "Could not parse user profile.".to_string())
    }

    pub async fn get_seating_info(&mut self) -> Result<SeatingInfo, String> {
        let p = [("menuId","655"),("url","studentProfilePESUAdmin"),("controllerMode","6404"),("actionType","5"),("id","0"),("selectedData","0")];
        let mut resp = self.req(Method::GET,"s/studentProfilePESUAdmin").query(&p).timeout(Duration::from_secs(25)).send().await.map_err(map_reqwest_error)?;
        if resp.status()==StatusCode::FORBIDDEN { self.refresh_csrf().await?; resp = self.req(Method::GET,"s/studentProfilePESUAdmin").query(&p).timeout(Duration::from_secs(25)).send().await.map_err(map_reqwest_error)?; }
        check_response_status(resp.status())?;
        Ok(parse_seating_html(&resp.text().await.map_err(map_reqwest_error)?))
    }
    pub async fn get_timetable(&mut self) -> Result<TimetableInfo, String> {
        let p = [("menuId","669"),("url","studentProfilePESUAdmin"),("controllerMode","6415"),("actionType","5"),("id","0"),("selectedData","0")];
        let mut resp = self.req(Method::GET,"s/studentProfilePESUAdmin").query(&p).timeout(Duration::from_secs(25)).send().await.map_err(map_reqwest_error)?;
        if resp.status()==StatusCode::FORBIDDEN { self.refresh_csrf().await?; resp = self.req(Method::GET,"s/studentProfilePESUAdmin").query(&p).timeout(Duration::from_secs(25)).send().await.map_err(map_reqwest_error)?; }
        check_response_status(resp.status())?;
        parse_timetable_html(&resp.text().await.map_err(map_reqwest_error)?)
    }
    pub async fn get_attendance(&mut self, batch_class_id: &str) -> Result<AttendanceInfo, String> {
        let p = [("menuId","660"),("controllerMode","6407"),("actionType","8"),("batchClassId",batch_class_id)];
        let mut resp = self.req(Method::GET,"s/studentProfilePESUAdmin").query(&p).timeout(Duration::from_secs(25)).send().await.map_err(map_reqwest_error)?;
        if resp.status()==StatusCode::FORBIDDEN { self.refresh_csrf().await?; resp = self.req(Method::GET,"s/studentProfilePESUAdmin").query(&p).timeout(Duration::from_secs(25)).send().await.map_err(map_reqwest_error)?; }
        let mut status = resp.status();
        let mut html = if status.is_success() { resp.text().await.map_err(map_reqwest_error)? } else { String::new() };
        let need_post = !status.is_success() || (!html.contains("subjetInfo") && !html.contains("Course Code"));
        if need_post {
            let fp = [("controllerMode","6407"),("actionType","8"),("batchClassId",batch_class_id),("menuId","660")];
            let pr = self.req(Method::POST,"s/studentProfilePESUAdmin").form(&fp).timeout(Duration::from_secs(25)).send().await.map_err(map_reqwest_error)?;
            status = pr.status(); check_response_status(status)?; html = pr.text().await.map_err(map_reqwest_error)?;
        } else { check_response_status(status)?; }
        let info = parse_attendance_html(&unescape_text(&html));
        if !info.records.is_empty() { return Ok(info); }
        let info2 = parse_attendance_html(&html);
        if !info2.records.is_empty() { return Ok(info2); }
        if html.contains("Data Not Available") || html.contains("No data") { return Ok(AttendanceInfo{records:vec![]}); }
        Ok(info)
    }
}
