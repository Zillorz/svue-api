use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::{api_request, ApiError};
use crate::crypto::AuthToken;

pub async fn schedule(token: &mut AuthToken, term_index: Option<i32>) -> Result<Schedule, ApiError> {
    let data: ClassListData = api_request(
        "StudentClassList",
        json!({
            "childIntID": 0,
            "loadAllTerms": true,
            "conSchOrgYearGU": "",
            "conSchTermIndex": "-1",
            "termIndex": "-1",
        }),
        token,
    )
    .await?;

    let all = data.student_class_schedule_for_all_terms;
    let index = match term_index {
        Some(i) => usize::try_from(i)
            .ok()
            .and_then(|i| all.term_lists.get(i))
            .ok_or(ApiError::NotFound)?
            .term_index
            .clone(),
        None => all.term_index,
    };

    let classes = all
        .student_class_schedule_for_terms
        .into_iter()
        .find(|t| t.this_term_index == index)
        .map(|t| t.class_lists)
        .unwrap_or_default();

    Ok(Schedule {
        classes: classes
            .into_iter()
            .map(|c| ClassSchedule {
                name: c.course_title,
                room: c.room_name,
                teacher: c.teacher,
                start_time: String::new(),
                end_time: String::new(),
                period: c.period,
            })
            .collect(),
        terms: all.term_lists.into_iter().map(|t| t.term_name).collect(),
    })
}

#[derive(Serialize)]
pub struct Schedule {
    classes: Vec<ClassSchedule>,
    terms: Vec<String>,
}

#[derive(Serialize)]
struct ClassSchedule {
    name: String,
    room: String,
    teacher: String,
    start_time: String,
    end_time: String,
    period: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClassListData {
    student_class_schedule_for_all_terms: AllTerms,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AllTerms {
    term_index: String,
    #[serde(default)]
    term_lists: Vec<Term>,
    #[serde(default)]
    student_class_schedule_for_terms: Vec<TermSchedule>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Term {
    term_index: String,
    term_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TermSchedule {
    this_term_index: String,
    #[serde(default)]
    class_lists: Vec<ClassEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClassEntry {
    period: String,
    course_title: String,
    room_name: String,
    teacher: String,
}
