use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::{api_request, ApiError};
use crate::crypto::AuthToken;

pub async fn get_attendance(token: &mut AuthToken) -> Result<Response, ApiError> {
    let data: AttendanceData =
        api_request("GetStudentAttendanceList", json!({"childIntID": 0}), token).await?;

    let absences: Vec<DayAbsence> = data
        .period_attendance
        .map(|p| p.absences)
        .unwrap_or_default()
        .into_iter()
        .map(|day| DayAbsence {
            date: day.absence_date,
            reason: day.reason,
            periods: day
                .periods
                .into_iter()
                .map(|p| PeriodAttendance {
                    absent: p.icon_name == "icon_unexcused.gif",
                    tardy: p.icon_name.contains("tardy"),
                    reason: if p.icon_name.is_empty() { String::new() } else { p.name },
                    name: p.course,
                    teacher: p.staff,
                })
                .collect(),
        })
        .collect();

    let mut summary: HashMap<String, ClassSummary> = HashMap::new();
    for period in absences.iter().flat_map(|d| &d.periods) {
        let entry = summary.entry(period.name.clone()).or_default();
        if period.absent {
            entry.absences += 1;
        }
        if period.tardy {
            entry.tardies += 1;
        }
        if !period.absent && !period.tardy && !period.reason.is_empty() {
            entry.excused += 1;
        }
    }

    Ok(Response { absences, summary })
}

#[derive(Serialize)]
pub struct Response {
    absences: Vec<DayAbsence>,
    summary: HashMap<String, ClassSummary>,
}

#[derive(Serialize)]
struct DayAbsence {
    date: String,
    reason: String,
    periods: Vec<PeriodAttendance>,
}

#[derive(Serialize)]
struct PeriodAttendance {
    absent: bool,
    tardy: bool,
    name: String,
    teacher: String,
    reason: String,
}

#[derive(Serialize, Default)]
struct ClassSummary {
    tardies: u32,
    absences: u32,
    excused: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttendanceData {
    period_attendance: Option<PeriodAttendanceData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PeriodAttendanceData {
    #[serde(default)]
    absences: Vec<Absence>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Absence {
    absence_date: String,
    reason: String,
    #[serde(default)]
    periods: Vec<Period>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Period {
    name: String,
    course: String,
    icon_name: String,
    staff: String,
}
