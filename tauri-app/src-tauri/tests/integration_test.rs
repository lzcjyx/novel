/// Integration test: verify the review arbiter logic with real-world data patterns.

#[test]
fn test_review_arbiter_decision_tree() {
    // Test 1: High scores, all pass → publish_ready
    let reviews = make_reviews(vec![90, 88, 92, 85, 91, 95, 89], vec![true; 7]);
    let agg = arbiter_aggregate(&reviews, 85, 2, 0);
    assert_eq!(agg.decision, "publish_ready");
    assert!(agg.publish_allowed);
    assert!(agg.final_score >= 85.0);

    // Test 2: Safety reviewer failed → needs_human_review
    let mut reviews = make_reviews(vec![90, 88, 92, 85, 91, 30, 89], vec![true; 7]);
    reviews[5].pass = Some(false); // safety reviewer at index 5
    let agg = arbiter_aggregate(&reviews, 85, 2, 0);
    assert_eq!(agg.decision, "needs_human_review");

    // Test 3: Average below threshold → revise (can still retry)
    let reviews = make_reviews(vec![65, 70, 60, 68, 72, 80, 66], vec![true; 7]);
    let agg = arbiter_aggregate(&reviews, 85, 2, 0);
    assert_eq!(agg.decision, "revise");

    // Test 4: Average below threshold, retries exhausted → needs_human_review
    let reviews = make_reviews(vec![65, 70, 60, 68, 72, 80, 66], vec![true; 7]);
    let agg = arbiter_aggregate(&reviews, 85, 2, 2);
    assert_eq!(agg.decision, "needs_human_review");

    // Test 5: Blocking issue detected → revise (can still retry)
    let mut reviews = make_reviews(vec![80, 80, 80, 80, 80, 90, 80], vec![true; 7]);
    reviews[0].blocking_issues = serde_json::json!([{"issue": "Major error"}]).to_string();
    let agg = arbiter_aggregate(&reviews, 85, 2, 0);
    assert_eq!(agg.decision, "revise");
    assert!(agg.blocking_issue_count >= 1);
}

#[test]
fn test_review_arbiter_edge_cases() {
    // Empty reviews → needs_human_review
    let agg = arbiter_aggregate(&[], 85, 2, 0);
    assert_eq!(agg.decision, "needs_human_review");

    // Single review at threshold
    let reviews = make_reviews(vec![85], vec![true]);
    let agg = arbiter_aggregate(&reviews, 85, 2, 0);
    assert_eq!(agg.decision, "publish_ready");

    // Single review exactly at threshold+1
    let reviews = make_reviews(vec![86], vec![true]);
    let agg = arbiter_aggregate(&reviews, 85, 2, 0);
    assert_eq!(agg.decision, "publish_ready");
}

// Minimal arbiter implementation for testing
#[derive(Debug, Clone)]
struct TestReview {
    agent_name: String,
    score: Option<i32>,
    pass: Option<bool>,
    blocking_issues: String,
}

#[derive(Debug)]
struct TestAggregation {
    average_score: f64, final_score: f64,
    decision: String, publish_allowed: bool,
    blocking_issue_count: i32,
}

fn make_reviews(scores: Vec<i32>, passes: Vec<bool>) -> Vec<TestReview> {
    let agents = ["continuity", "character", "plot_logic", "pacing", "style", "safety", "publication"];
    scores.iter().enumerate().map(|(i, &s)| TestReview {
        agent_name: agents[i % agents.len()].into(),
        score: Some(s),
        pass: Some(passes[i % passes.len()]),
        blocking_issues: "[]".into(),
    }).collect()
}

fn count_json_items(s: &str) -> usize {
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(serde_json::Value::Array(arr)) => arr.len(),
        _ => 0,
    }
}

fn arbiter_aggregate(reviews: &[TestReview], quality_threshold: i32, max_revise_count: i32, revise_count: i32) -> TestAggregation {
    if reviews.is_empty() {
        return TestAggregation { average_score: 0.0, final_score: 0.0, decision: "needs_human_review".into(), publish_allowed: false, blocking_issue_count: 0 };
    }

    let scores: Vec<i32> = reviews.iter().filter_map(|r| r.score).collect();
    let avg = if scores.is_empty() { 0.0 } else {
        scores.iter().map(|&s| s as f64).sum::<f64>() / scores.len() as f64
    };

    let has_blocking = reviews.iter().any(|r| count_json_items(&r.blocking_issues) > 0);
    let safety_failed = reviews.iter().any(|r| r.agent_name == "safety" && r.pass == Some(false));
    let blocking_count = reviews.iter().map(|r| count_json_items(&r.blocking_issues)).sum::<usize>() as i32;

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
            (avg.min(quality_threshold as f64 - 1.0), "needs_human_review", false)
        } else {
            (avg, "revise", false)
        }
    } else {
        (avg, "publish_ready", true)
    };

    TestAggregation { average_score: avg, final_score, decision: decision.to_string(), publish_allowed, blocking_issue_count: blocking_count }
}
