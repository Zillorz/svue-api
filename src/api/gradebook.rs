use std::collections::HashMap;
use std::num::ParseFloatError;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::api::{api_request, ApiError, ProcessWebServiceRequest};
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
    let params = rp
        .map(|x| format!("<ReportPeriod>{x}</ReportPeriod>"))
        .unwrap_or_default();

    let result = api_request(
        ProcessWebServiceRequest::ck_default("Gradebook".to_string(), params, token),
        token,
    )
    .await?;

    let gb: Gradebook = quick_xml::de::from_str(result.as_str())?;
    Ok(gb.try_into()?)
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
            .report_period
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
                .course
                .into_iter()
                .map(|c| {
                    // necessary for some odd classes ig?
                    let Some(mark) = c.marks.mark else {
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

                    if let Some(agcs) = mark.grade_calculation_summary.assignment_grade_calc {
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
                    for assign in mark.assignments.assignment {
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

// XML structs
#[derive(Serialize, Deserialize, Debug)]
struct Gradebook {
    #[serde(rename = "@xmlns:xsd")]
    xmlns_xsd: String,
    #[serde(rename = "@xmlns:xsi")]
    xmlns_xsi: String,
    #[serde(rename = "@Type")]
    gradebook_type: String,
    #[serde(rename = "@ErrorMessage")]
    error_message: String,
    #[serde(rename = "@HideStandardGraphInd")]
    hide_standard_graph_ind: String,
    #[serde(rename = "@HideMarksColumnElementary")]
    hide_marks_column_elementary: String,
    #[serde(rename = "@HidePointsColumnElementary")]
    hide_points_column_elementary: String,
    #[serde(rename = "@HidePercentSecondary")]
    hide_percent_secondary: String,
    #[serde(rename = "@DisplayStandardsData")]
    display_standards_data: String,
    #[serde(rename = "@GBStandardsTabDefault")]
    gbstandards_tab_default: String,
    #[serde(rename = "ReportingPeriods")]
    reporting_periods: ReportingPeriods,
    #[serde(rename = "ReportingPeriod")]
    reporting_period: ReportingPeriod_,
    #[serde(rename = "Courses")]
    courses: Courses,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReportingPeriods {
    #[serde(rename = "ReportPeriod")]
    report_period: Vec<ReportPeriod>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReportPeriod {
    #[serde(rename = "@Index")]
    index: String,
    #[serde(rename = "@GradePeriod")]
    grade_period: String,
    #[serde(rename = "@StartDate")]
    start_date: String,
    #[serde(rename = "@EndDate")]
    end_date: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReportingPeriod_ {
    #[serde(rename = "@GradePeriod")]
    grade_period: String,
    #[serde(rename = "@StartDate")]
    start_date: String,
    #[serde(rename = "@EndDate")]
    end_date: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Courses {
    #[serde(rename = "Course")]
    course: Vec<Course>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Course {
    #[serde(rename = "@UsesRichContent")]
    uses_rich_content: String,
    #[serde(rename = "@Period")]
    period: String,
    #[serde(rename = "@Title")]
    title: String,
    #[serde(rename = "@CourseName")]
    name: String,
    #[serde(rename = "@Room")]
    room: String,
    #[serde(rename = "@Staff")]
    staff: String,
    #[serde(rename = "@StaffEMail")]
    staff_email: String,
    #[serde(rename = "@StaffGU")]
    staff_gu: String,
    #[serde(rename = "@ImageType")]
    image_type: String,
    #[serde(rename = "@HighlightPercentageCutOffForProgressBar")]
    highlight_percentage_cut_off_for_progress_bar: String,
    #[serde(rename = "Marks")]
    marks: Marks,
}

#[derive(Serialize, Deserialize, Debug)]
struct Marks {
    #[serde(rename = "Mark")]
    mark: Option<Mark>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Mark {
    #[serde(rename = "@MarkName")]
    mark_name: String,
    #[serde(rename = "@CalculatedScoreString")]
    calculated_score_string: String,
    #[serde(rename = "@CalculatedScoreRaw")]
    calculated_score_raw: String,
    #[serde(rename = "StandardViews")]
    standard_views: StandardViews,
    #[serde(rename = "GradeCalculationSummary")]
    grade_calculation_summary: GradeCalculationSummary,
    #[serde(rename = "Assignments")]
    assignments: Assignments,
}

#[derive(Serialize, Deserialize, Debug)]
struct StandardViews {}

#[derive(Serialize, Deserialize, Debug)]
struct GradeCalculationSummary {
    #[serde(rename = "AssignmentGradeCalc")]
    assignment_grade_calc: Option<Vec<AssignmentGradeCalc>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AssignmentGradeCalc {
    #[serde(rename = "@Type")]
    assignment_grade_calc_type: String,
    #[serde(rename = "@Weight")]
    weight: String,
    #[serde(rename = "@Points")]
    points: String,
    #[serde(rename = "@PointsPossible")]
    points_possible: String,
    #[serde(rename = "@WeightedPct")]
    weighted_pct: String,
    #[serde(rename = "@CalculatedMark")]
    calculated_mark: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Assignments {
    #[serde(rename = "Assignment")]
    #[serde(default)]
    assignment: Vec<Assignment_>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Assignment_ {
    #[serde(rename = "@GradebookID")]
    gradebook_id: String,
    #[serde(rename = "@Measure")]
    measure: String,
    #[serde(rename = "@Type")]
    assignment_type: String,
    #[serde(rename = "@Date")]
    date: String,
    #[serde(rename = "@DueDate")]
    due_date: String,
    #[serde(rename = "@DisplayScore")]
    display_score: Option<String>,
    #[serde(rename = "@ScoreCalValue")]
    score_cal_value: Option<String>,
    #[serde(rename = "@TimeSincePost")]
    time_since_post: String,
    #[serde(rename = "@TotalSecondsSincePost")]
    total_seconds_since_post: String,
    #[serde(rename = "@ScoreMaxValue")]
    score_max_value: Option<String>,
    #[serde(rename = "@ScoreType")]
    score_type: Option<String>,
    #[serde(rename = "@Points")]
    points: String,
    #[serde(rename = "@Point")]
    points_earned: Option<String>,
    #[serde(rename = "@PointPossible")]
    points_possible: Option<String>,
    #[serde(rename = "@Notes")]
    notes: String,
    #[serde(rename = "@TeacherID")]
    teacher_id: String,
    #[serde(rename = "@StudentID")]
    student_id: String,
    #[serde(rename = "@MeasureDescription")]
    measure_description: String,
    #[serde(rename = "@HasDropBox")]
    has_drop_box: String,
    #[serde(rename = "@DropStartDate")]
    drop_start_date: String,
    #[serde(rename = "@DropEndDate")]
    drop_end_date: String,
    #[serde(rename = "$text")]
    text: Option<String>,
    #[serde(rename = "Resources")]
    resources: Resources,
    #[serde(rename = "Standards")]
    standards: Standards,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resources {}

#[derive(Serialize, Deserialize, Debug)]
struct Standards {}
