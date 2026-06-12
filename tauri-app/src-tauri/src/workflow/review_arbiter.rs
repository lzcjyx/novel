use crate::models::{AgentReview, ReviewAggregation};

pub fn aggregate_reviews(
    reviews: &[AgentReview],
    quality_threshold: i32,
    max_revise_count: i32,
    revise_count: i32,
) -> ReviewAggregation {
    let scores: Vec<i32> = reviews.iter().filter_map(|r| r.score).collect();
    let avg = if scores.is_empty() {
        0.0
    } else {
        scores.iter().map(|&s| s as f64).sum::<f64>() / scores.len() as f64
    };

    // Edge case: no reviews at all
    if reviews.is_empty() {
        return ReviewAggregation {
            average_score: 0.0,
            final_score: 0.0,
            decision: "needs_human_review".into(),
            publish_allowed: false,
            blocking_issue_count: 0,
            all_passed: true,
            safety_passed: true,
            reviews: vec![],
        };
    }

    // Robust blocking detection: parse as JSON array and count actual items
    let has_blocking = reviews
        .iter()
        .any(|r| count_json_items(&r.blocking_issues) > 0);

    let safety_failed = reviews
        .iter()
        .any(|r| r.agent_name == "safety_reviewer" && r.pass == Some(false));

    let blocking_count = reviews
        .iter()
        .map(|r| count_json_items(&r.blocking_issues))
        .sum::<usize>() as i32;

    let (final_score, decision, publish_allowed) = if safety_failed {
        (avg.min(40.0), "needs_human_review", false)
    } else if has_blocking {
        if revise_count >= max_revise_count {
            (avg.min(74.0), "needs_human_review", false)
        } else {
            (avg.min(74.0), "revise", false)
        }
    } else if avg < quality_threshold as f64 {
        if revise_count >= max_revise_count {
            (
                avg.min(quality_threshold as f64 - 1.0),
                "needs_human_review",
                false,
            )
        } else {
            (avg, "revise", false)
        }
    } else {
        (avg, "publish_ready", true)
    };

    ReviewAggregation {
        average_score: avg,
        final_score,
        decision: decision.to_string(),
        publish_allowed,
        blocking_issue_count: blocking_count,
        all_passed: reviews.iter().all(|r| r.pass == Some(true)),
        safety_passed: !safety_failed,
        reviews: reviews.to_vec(),
    }
}

/// Count items in a JSON array string robustly (handles null, empty, whitespace, malformed)
fn count_json_items(s: &str) -> usize {
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(serde_json::Value::Array(arr)) => arr.len(),
        Ok(serde_json::Value::Null) => 0,
        _ => 0, // string "[]", whitespace, or malformed → no items
    }
}
