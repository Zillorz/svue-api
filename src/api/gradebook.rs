use std::collections::HashMap;
use std::num::ParseFloatError;

use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::api::{api_request, ApiError};
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

pub async fn get_grade_book(token: &mut AuthToken, rp: Option<i32>) -> Result<Response, ApiError> {
    let data: GradebookData = api_request(
        "Gradebook",
        json!({
            "reportPeriod": rp.map(|x| x.to_string()).unwrap_or_default(),
            "concurrentSchOrgYearGU": "",
            "childIntID": 0,
            "languageCode": "en",
        }),
        token,
    )
    .await?;

    Ok(data.traditional_gradebook.try_into()?)
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
                            category: c.image_type.unwrap_or_default(),
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

                    for agc in mark.grade_calculation_summary {
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
                        category: c.image_type.unwrap_or_default(),
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GradebookData {
    traditional_gradebook: Gradebook,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Gradebook {
    reporting_periods: Vec<ReportPeriod>,
    reporting_period: ReportPeriod,
    courses: Vec<Course>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportPeriod {
    grade_period: String,
    start_date: String,
    end_date: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Course {
    title: String,
    staff: String,
    image_type: Option<String>,
    #[serde(default)]
    marks: Vec<Mark>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Mark {
    calculated_score_string: String,
    calculated_score_raw: String,
    #[serde(default)]
    grade_calculation_summary: Vec<AssignmentGradeCalc>,
    #[serde(default)]
    assignments: Vec<Assignment_>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentGradeCalc {
    #[serde(rename = "type")]
    assignment_grade_calc_type: String,
    weight: String,
    points: String,
    points_possible: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Assignment_ {
    measure: String,
    #[serde(rename = "type")]
    assignment_type: String,
    score_cal_value: Option<String>,
    score_max_value: Option<String>,
    points: String,
    #[serde(default)]
    notes: String,
}
