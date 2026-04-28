// Data types matching `lcs scan -f json` output (lcs >= 0.5.3).
// Every field is required by the contract — no Option<> for fields that the
// 11.5 surface (and the 0.5.3 clean-response fix) guarantees.

use crate::corpus::sample::Severity;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ScanReport {
    pub clean: bool,
    pub finding_count: u32,
    pub findings: Vec<Finding>,
    pub rule_set_fingerprint: String,
    pub threat_scores: ThreatScores,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Finding {
    pub category: String,
    pub severity: Severity,
    pub description: String,
    pub matched_text: String,
    pub byte_range: (usize, usize),
    pub rule_name: String,
    pub engine: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ThreatScores {
    pub class_scores: BTreeMap<String, i64>,
    pub cumulative: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLEAN_FIXTURE: &str = r#"{
      "clean": true,
      "finding_count": 0,
      "findings": [],
      "rule_set_fingerprint": "4c6cd18ac803ea92cb145a143b6e1629b30ee655e59afa6f60a65f150c11469a",
      "threat_scores": {
        "class_scores": {},
        "cumulative": 0
      }
    }"#;

    const SINGLE_FINDING_FIXTURE: &str = r#"{
      "clean": false,
      "finding_count": 1,
      "findings": [{
        "category": "prompt_injection",
        "severity": "critical",
        "description": "Instruction override: ignore previous instructions",
        "matched_text": "Ignore all previous instructions",
        "byte_range": [0, 32],
        "rule_name": "prompt_injection",
        "engine": "simple"
      }],
      "rule_set_fingerprint": "4c6cd18ac803ea92cb145a143b6e1629b30ee655e59afa6f60a65f150c11469a",
      "threat_scores": {
        "class_scores": {"prompt_injection": 1},
        "cumulative": 1
      }
    }"#;

    const MULTI_FINDING_FIXTURE: &str = r#"{
      "clean": false,
      "finding_count": 2,
      "findings": [
        {
          "category": "prompt_injection",
          "severity": "critical",
          "description": "Instruction override attempting to bypass previous context",
          "matched_text": "Ignore all previous instructions",
          "byte_range": [0, 32],
          "rule_name": "prompt_injection_critical",
          "engine": "syara"
        },
        {
          "category": "refusal_suppression",
          "severity": "critical",
          "description": "Imperative directive to suppress safety refusals",
          "matched_text": "Do not refuse",
          "byte_range": [357, 370],
          "rule_name": "refusal_suppression_critical",
          "engine": "syara"
        }
      ],
      "rule_set_fingerprint": "bb3ce91b0d6816f3676831c3f049f3c69a75425be727dae7467aff4d08f511c1",
      "threat_scores": {
        "class_scores": {"prompt_hijack": 5, "social_engineering": 5},
        "cumulative": 10
      }
    }"#;

    #[test]
    fn parses_clean_response_with_empty_threat_scores() {
        let report: ScanReport = serde_json::from_str(CLEAN_FIXTURE).expect("parse");
        assert!(report.clean);
        assert_eq!(report.finding_count, 0);
        assert!(report.findings.is_empty());
        assert_eq!(
            report.rule_set_fingerprint,
            "4c6cd18ac803ea92cb145a143b6e1629b30ee655e59afa6f60a65f150c11469a"
        );
        assert!(report.threat_scores.class_scores.is_empty());
        assert_eq!(report.threat_scores.cumulative, 0);
    }

    #[test]
    fn parses_single_finding_with_byte_range_tuple() {
        let report: ScanReport = serde_json::from_str(SINGLE_FINDING_FIXTURE).expect("parse");
        assert!(!report.clean);
        assert_eq!(report.finding_count, 1);
        let f = &report.findings[0];
        assert_eq!(f.category, "prompt_injection");
        assert_eq!(f.severity, Severity::Critical);
        assert_eq!(f.matched_text, "Ignore all previous instructions");
        assert_eq!(f.byte_range, (0, 32));
        assert_eq!(f.rule_name, "prompt_injection");
        assert_eq!(f.engine, "simple");
        assert_eq!(report.threat_scores.cumulative, 1);
        assert_eq!(
            report.threat_scores.class_scores.get("prompt_injection"),
            Some(&1)
        );
    }

    #[test]
    fn parses_multi_finding_response() {
        let report: ScanReport = serde_json::from_str(MULTI_FINDING_FIXTURE).expect("parse");
        assert_eq!(report.finding_count, 2);
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0].category, "prompt_injection");
        assert_eq!(report.findings[1].category, "refusal_suppression");
        assert_eq!(report.findings[1].byte_range, (357, 370));
        assert_eq!(report.threat_scores.cumulative, 10);
        assert_eq!(report.threat_scores.class_scores.len(), 2);
    }

    #[test]
    fn rejects_garbage_input() {
        let result: Result<ScanReport, _> = serde_json::from_str("not json");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_required_field() {
        // Missing rule_set_fingerprint must fail to parse — it's required by lcs >= 0.5.2.
        let bad = r#"{
          "clean": true, "finding_count": 0, "findings": [],
          "threat_scores": {"class_scores": {}, "cumulative": 0}
        }"#;
        let result: Result<ScanReport, _> = serde_json::from_str(bad);
        assert!(result.is_err());
    }
}
