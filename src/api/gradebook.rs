use std::collections::HashMap;
use std::num::ParseFloatError;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::api::{ApiError, api_req};
use crate::crypto::AuthToken;

#[derive(Error, Debug)]
pub enum GradebookError {
    #[error("Missing field '{0}'")]
    MissingField(&'static str),
    #[error("Bad float")]
    NumParsing(#[from] ParseFloatError),
    #[error("Bad point_string")]
    InvalidPointString,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GradebookReq {
    #[serde(skip_serializing_if = "Option::is_none")]
    report_period: Option<i32>,
    #[serde(rename = "concurrentSchOrgYearGU")]
    concurrent_sch_org_year_gu: String,
    #[serde(rename = "childIntID")]
    child_int_id: i32,
    langauge_code: String,
}

pub async fn get_grade_book(token: &mut AuthToken, rp: Option<i32>) -> Result<Response, ApiError> {
    let result: OuterGb = api_req(
        "Gradebook",
        GradebookReq {
            report_period: rp,
            concurrent_sch_org_year_gu: String::default(),
            child_int_id: 0,
            langauge_code: "ENG".to_string(),
        },
        token,
    )
    .await?;

    Ok(result.inner.try_into()?)
}

// API structs
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Response {
    classes: Vec<Class>,
    pub report_period: i32,
    pub reporting_periods: Vec<ReportingPeriod>,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct ReportingPeriod {
    pub name: String,
    start_date: String,
    end_date: String,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
struct Class {
    name: String,
    teacher: String,
    category: String,
    grade: f32,
    letter_grade: String,
    categories: HashMap<String, Category>,
    assignments: Vec<Assignment>,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
struct Category {
    weight: f32,
    points_earned: f32,
    points_possible: f32,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
struct Assignment {
    name: String,
    kind: String,
    points_earned: f32,
    points_possible: f32,
    #[serde(skip_serializing_if = "String::is_empty")]
    notes: String,
}

impl TryFrom<Gradebook> for Response {
    type Error = GradebookError;

    fn try_from(value: Gradebook) -> Result<Self, Self::Error> {
        let reporting_periods: Vec<ReportingPeriod> = value
            .reporting_periods
            .into_iter()
            .map(|p| ReportingPeriod {
                name: p.grade_period,
                start_date: p.start_date,
                end_date: p.end_date,
            })
            .collect();

        let rp = reporting_periods
            .iter()
            .position(|r| r.name == value.reporting_period.grade_period)
            .ok_or(GradebookError::MissingField("gp_idx"))?;

        Ok(Response {
            classes: value
                .courses
                .into_iter()
                .map(|c| {
                    // necessary for some odd classes ig?
                    let Some(mark) = c.marks.into_iter().next() else {
                        return Ok::<_, GradebookError>(Class {
                            name: c.title,
                            teacher: c.staff,
                            grade: 0.0,
                            category: c.image_type,
                            letter_grade: "N/A".to_string(),
                            assignments: Vec::new(),
                            categories: HashMap::new(),
                        });
                    };

                    let grade: f32 = mark.calculated_score_raw.parse()?;
                    let mut lg = mark.calculated_score_string;

                    if lg.chars().any(char::is_numeric) {
                        lg = match grade {
                            x if x >= 89.5 => "A",
                            x if x >= 79.5 => "B",
                            x if x >= 69.5 => "C",
                            x if x >= 59.5 => "D",
                            x if x.is_finite() => "E",
                            _ => "N/A",
                        }
                        .to_string();
                    }

                    let mut categories = HashMap::new();

                    if let Some(agcs) = mark.grade_calculation_summary {
                        for agc in agcs {
                            if agc.assignment_grade_calc_type == "TOTAL" {
                                continue;
                            }

                            categories.insert(
                                agc.assignment_grade_calc_type,
                                Category {
                                    weight: agc.weight.trim_matches('%').parse::<f32>()? / 100.0,
                                    points_earned: agc.points.replace(",", "").parse()?,
                                    points_possible: agc
                                        .points_possible
                                        .replace(",", "")
                                        .parse()?,
                                },
                            );
                        }
                    }

                    let mut assignments = Vec::new();
                    for assign in mark.assignments {
                        let points_earned = assign
                            .score_cal_value
                            .and_then(|x| x.replace(",", "").parse().ok())
                            .unwrap_or(f32::NAN);

                        let Some(points_possible) = assign
                            .score_max_value
                            .and_then(|x| x.replace(",", "").parse::<f32>().ok())
                            .or_else(|| {
                                if assign.points.contains("/") {
                                    assign
                                        .points
                                        .split_once('/')
                                        .and_then(|x| x.1.trim().replace(",", "").parse().ok())
                                } else {
                                    assign
                                        .points
                                        .replace("Points Possible", "")
                                        .replace(",", "")
                                        .trim()
                                        .parse()
                                        .ok()
                                }
                            })
                        else {
                            continue;
                        };

                        fn unescape_xml(inp: String) -> String {
                            inp.replace("&apos;", "'")
                                .replace("&quot;", "\"")
                                .replace("&amp;", "&")
                                .replace("&lt;", "<")
                                .replace("&gt;", ">")
                        }

                        assignments.push(Assignment {
                            name: unescape_xml(assign.measure),
                            kind: assign.assignment_type,
                            points_earned,
                            points_possible,
                            notes: assign.notes,
                        })
                    }

                    Ok::<_, GradebookError>(Class {
                        name: c.title,
                        teacher: c.staff,
                        grade,
                        category: c.image_type,
                        letter_grade: lg,
                        assignments,
                        categories,
                    })
                })
                .collect::<Result<Vec<_>, GradebookError>>()?,
            report_period: rp as i32,
            reporting_periods,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct OuterGb {
    #[serde(rename = "traditionalGradebook")]
    inner: Gradebook,
}

// XML structs
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Gradebook {
    #[serde(rename = "type")]
    gradebook_type: String,
    #[serde(rename = "errorMessage")]
    error_message: String,
    // #[serde(rename = "hideStandardGraphInd")]
    // hide_standard_graph_ind: String,
    // #[serde(rename = "hideMarksColumnElementary")]
    // hide_marks_column_elementary: String,
    // #[serde(rename = "hidePointsColumnElementary")]
    // hide_points_column_elementary: String,
    // #[serde(rename = "hidePercentSecondary")]
    // hide_percent_secondary: String,
    // #[serde(rename = "displayStandardsData")]
    // display_standards_data: String,
    // #[serde(rename = "gbStandardsTabDefault")]
    // gbstandards_tab_default: String,
    reporting_periods: Vec<ReportPeriod>,
    reporting_period: ReportPeriod,
    courses: Vec<Course>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ReportPeriod {
    index: String,
    grade_period: String,
    start_date: String,
    end_date: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Course {
    period: String,
    title: String,
    #[serde(rename = "courseName")]
    name: String,
    room: String,
    staff: String,
    #[serde(rename = "staffEMail")]
    staff_email: String,
    #[serde(rename = "staffGU")]
    staff_gu: String,
    image_type: String,
    highlight_percentage_cut_off_for_progress_bar: i32,
    marks: Vec<Mark>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Mark {
    mark_name: String,
    calculated_score_string: String,
    calculated_score_raw: String,
    grade_calculation_summary: Option<Vec<AssignmentGradeCalc>>,
    #[serde(default)]
    assignments: Vec<Assignment_>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AssignmentGradeCalc {
    #[serde(rename = "type")]
    assignment_grade_calc_type: String,
    weight: String,
    points: String,
    points_possible: String,
    weighted_pct: String,
    calculated_mark: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Assignment_ {
    #[serde(rename = "gradebookID")]
    gradebook_id: i64,
    measure: String,
    #[serde(rename = "type")]
    assignment_type: String,
    date: String,
    due_date: String,
    display_score: Option<String>,
    score_cal_value: Option<String>,
    time_since_post: String,
    total_seconds_since_post: f64,
    score_max_value: Option<String>,
    score_type: Option<String>,
    points: String,
    #[serde(rename = "point")]
    points_earned: Option<String>,
    points_possible: Option<String>,
    notes: String,
    #[serde(rename = "teacherID")]
    teacher_id: i64,
    #[serde(rename = "studentID")]
    student_id: i64,
    measure_description: String,
    has_drop_box: bool,
    drop_start_date: String,
    drop_end_date: String,
}
