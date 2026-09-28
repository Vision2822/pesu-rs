// Source: https://v2.tauri.app/develop/calling-rust/
// Source: https://v2.tauri.app/develop/state-management/
// Source: https://docs.rs/keyring/3.6.3/keyring/struct.Entry.html
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod scraper;

use scraper::{AttendanceInfo, Course, LoginResponse, PESUScraper, SeatingInfo, Semester, TimetableInfo, UserProfile};
use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

const KEYRING_SERVICE: &str = "pesu-rs";
const KEYRING_LAST_SRN_KEY: &str = "saved_srn";

pub struct AppState {
    pub scraper: Mutex<PESUScraper>,
    pub semesters: Mutex<Vec<Semester>>,
    pub selected_semester: Mutex<Option<String>>,
    pub courses: Mutex<Vec<Course>>,
}

#[derive(Serialize)]
pub struct SavedCredentials {
    pub srn: String,
    pub has_password: bool,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Backend connected: Hello, {}!", name)
}

fn delete_entry_sync(service: &str, user: &str) -> Result<(), String> {
    let entry = match keyring::Entry::new(service, user) {
        Ok(e) => e,
        Err(keyring::Error::NoEntry) => return Ok(()),
        Err(e) => return Err(format!("Keyring error on {user}: {e}")),
    };
    match entry.delete_credential() {
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete {user} from vault: {e}")),
    }
}

pub fn delete_credentials_sync() -> Result<(), String> {
    let saved_srn = match keyring::Entry::new(KEYRING_SERVICE, KEYRING_LAST_SRN_KEY) {
        Ok(e) => match e.get_password() {
            Ok(pwd) => Some(pwd),
            Err(keyring::Error::NoEntry) => None,
            Err(e) => return Err(format!("Keyring error reading saved SRN: {e}")),
        },
        Err(keyring::Error::NoEntry) => None,
        Err(e) => return Err(format!("Keyring initialization error: {e}")),
    };
    if let Some(srn) = saved_srn {
        let trimmed = srn.trim();
        if !trimmed.is_empty() {
            delete_entry_sync(KEYRING_SERVICE, trimmed)?;
        }
    }
    delete_entry_sync(KEYRING_SERVICE, KEYRING_LAST_SRN_KEY)?;
    Ok(())
}

#[tauri::command]
async fn get_saved_credentials() -> Result<Option<SavedCredentials>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let srn_entry = match keyring::Entry::new(KEYRING_SERVICE, KEYRING_LAST_SRN_KEY) {
            Ok(e) => e,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(e) => return Err(format!("Keyring error: {e}")),
        };
        let srn = match srn_entry.get_password() {
            Ok(s) => s,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(e) => return Err(format!("Keyring error reading SRN: {e}")),
        };
        let trimmed = srn.trim().to_string();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let pwd_entry = match keyring::Entry::new(KEYRING_SERVICE, &trimmed) {
            Ok(e) => e,
            Err(keyring::Error::NoEntry) => {
                return Ok(Some(SavedCredentials {
                    srn: trimmed,
                    has_password: false,
                }))
            }
            Err(e) => return Err(format!("Keyring error reading password entry: {e}")),
        };
        let has_password = match pwd_entry.get_password() {
            Ok(_) => true,
            Err(keyring::Error::NoEntry) => false,
            Err(e) => return Err(format!("Keyring error checking password: {e}")),
        };
        Ok(Some(SavedCredentials {
            srn: trimmed,
            has_password,
        }))
    })
    .await
    .map_err(|e| format!("Task execution failed: {e}"))?
}

#[tauri::command]
async fn forget_saved_credentials() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(delete_credentials_sync)
        .await
        .map_err(|e| format!("Task execution failed: {e}"))?
}

#[tauri::command]
async fn login(
    srn: String,
    password: String,
    remember_me: Option<bool>,
    state: State<'_, AppState>,
) -> Result<LoginResponse, String> {
    let trimmed_srn = srn.trim().to_uppercase();
    if trimmed_srn.is_empty() {
        return Err("SRN cannot be empty.".to_string());
    }
    let actual_password = if password.is_empty() {
        let srn_lookup = trimmed_srn.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let pwd_entry = keyring::Entry::new(KEYRING_SERVICE, &srn_lookup)
                .map_err(|_| "Could not access OS credential vault.".to_string())?;
            pwd_entry
                .get_password()
                .map_err(|_| "No saved password found. Please enter your password.".to_string())
        })
        .await
        .map_err(|e| format!("Task execution failed: {e}"))??
    } else {
        password
    };
    let mut scraper = state.scraper.lock().await;
    let res = scraper.login(&trimmed_srn, &actual_password).await?;
    let remember = remember_me.unwrap_or(false);
    if remember {
        let srn_to_save = trimmed_srn.clone();
        let pwd_to_save = actual_password.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let srn_entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_LAST_SRN_KEY)
                .map_err(|e| format!("Keyring error: {e}"))?;
            srn_entry
                .set_password(&srn_to_save)
                .map_err(|e| format!("Failed to save SRN: {e}"))?;
            let pwd_entry = keyring::Entry::new(KEYRING_SERVICE, &srn_to_save)
                .map_err(|e| format!("Keyring error: {e}"))?;
            pwd_entry
                .set_password(&pwd_to_save)
                .map_err(|e| format!("Failed to save password: {e}"))?;
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Task execution failed: {e}"))??;
    } else {
        tauri::async_runtime::spawn_blocking(delete_credentials_sync)
            .await
            .map_err(|e| format!("Task execution failed: {e}"))??;
    }
    Ok(res)
}

#[tauri::command]
async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let mut scraper = state.scraper.lock().await;
    *scraper = PESUScraper::new()?;
    drop(scraper);
    *state.semesters.lock().await = Vec::new();
    *state.selected_semester.lock().await = None;
    *state.courses.lock().await = Vec::new();
    Ok(())
}

#[tauri::command]
async fn get_user_profile(state: State<'_, AppState>) -> Result<UserProfile, String> {
    let scraper = state.scraper.lock().await;
    scraper.get_user_profile().await
}

#[tauri::command]
async fn get_semesters(state: State<'_, AppState>) -> Result<Vec<Semester>, String> {
    let scraper = state.scraper.lock().await;
    let sems = scraper.get_semesters().await?;
    let mut stored = state.semesters.lock().await;
    *stored = sems.clone();
    Ok(sems)
}

#[tauri::command]
async fn set_selected_semester(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut sel = state.selected_semester.lock().await;
    *sel = if id.trim().is_empty() {
        None
    } else {
        Some(id.trim().to_string())
    };
    Ok(())
}

#[tauri::command]
async fn get_courses(
    semester_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Course>, String> {
    let scraper = state.scraper.lock().await;
    let fetched = scraper.get_courses(&semester_id).await?;
    let mut stored = state.courses.lock().await;
    *stored = fetched.clone();
    Ok(fetched)
}

#[tauri::command]
async fn get_seating(state: State<'_, AppState>) -> Result<SeatingInfo, String> {
    let mut scraper = state.scraper.lock().await;
    scraper.get_seating_info().await
}

#[tauri::command]
async fn get_timetable(state: State<'_, AppState>) -> Result<TimetableInfo, String> {
    let mut scraper = state.scraper.lock().await;
    scraper.get_timetable().await
}

#[tauri::command]
async fn get_attendance(
    semester_id: String,
    state: State<'_, AppState>,
) -> Result<AttendanceInfo, String> {
    let mut scraper = state.scraper.lock().await;
    // semester_id is batchClassId
    scraper.get_attendance(&semester_id).await
}

fn main() {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls ring crypto provider");
    }
    let scraper_instance = PESUScraper::new().expect("Failed to initialize scraper");
    tauri::Builder::default()
        .manage(AppState {
            scraper: Mutex::new(scraper_instance),
            semesters: Mutex::new(Vec::new()),
            selected_semester: Mutex::new(None),
            courses: Mutex::new(Vec::new()),
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            login,
            logout,
            get_saved_credentials,
            forget_saved_credentials,
            get_user_profile,
            get_semesters,
            set_selected_semester,
            get_courses,
            get_seating,
            get_timetable,
            get_attendance
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
