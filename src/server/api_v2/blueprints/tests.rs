//! Tests extracted from `mod.rs` for Code Health scoring.

use crate::config::SITE_ORIGIN;
use std::collections::HashMap;

use super::*;
use serde_json::json;

#[test]
fn validate_title_defaults_empty_to_untitled() {
    assert_eq!(validate_title("  ").unwrap(), "Untitled blueprint");
}

#[test]
fn validate_title_rejects_overlong_input() {
    let long = "a".repeat(MAX_TITLE_LEN + 1);
    assert!(validate_title(&long).is_err());
}

#[test]
fn validate_nodes_normalizes_tool_and_note() {
    let nodes = json!([
        {"id": "n1", "kind": "tool", "slug": "  foo  ", "chains": ["Base", "base"], "x": 10, "y": 20},
        {"id": "n2", "kind": "note", "text": "hello", "x": 0, "y": 0}
    ]);
    let result = validate_nodes(&nodes).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["slug"], "foo");
    assert_eq!(arr[0]["chains"], json!(["base"]));
}

#[test]
fn validate_nodes_rejects_invalid_kind() {
    let nodes = json!([{"id": "n1", "kind": "widget", "x": 0, "y": 0}]);
    assert!(validate_nodes(&nodes).is_err());
}

#[test]
fn validate_nodes_rejects_out_of_range_coordinates() {
    let nodes = json!([{"id": "n1", "kind": "note", "text": "", "x": -1, "y": 0}]);
    assert!(validate_nodes(&nodes).is_err());
}

#[test]
fn validate_nodes_normalizes_chain() {
    let nodes = json!([
        {"id": "c1", "kind": "chain", "chainId": "  ethereum  ", "x": 8, "y": 16}
    ]);
    let result = validate_nodes(&nodes).unwrap();
    assert_eq!(result[0]["chainId"], "ethereum");
}

#[test]
fn validate_edges_accepts_solid_and_arrow() {
    let nodes = json!([
        {"id": "a", "kind": "tool", "slug": "foo", "x": 0, "y": 0},
        {"id": "b", "kind": "note", "text": "", "x": 40, "y": 40}
    ]);
    let node_ids = node_ids_from_value(&validate_nodes(&nodes).unwrap()).unwrap();
    let edges = json!([
        {
            "id": "e1",
            "fromId": "a",
            "toId": "b",
            "style": "arrow",
            "color": "#E76F00"
        }
    ]);
    let result = validate_edges(&edges, &node_ids).unwrap();
    assert_eq!(result[0]["style"], "arrow");
}

#[test]
fn validate_edges_rejects_unknown_nodes() {
    let edges = json!([
        {
            "id": "e1",
            "fromId": "missing",
            "toId": "also-missing",
            "style": "solid",
            "color": "#1A1A1A"
        }
    ]);
    assert!(validate_edges(&edges, &[]).is_err());
}

#[test]
fn build_agent_export_markdown_includes_tools_notes_flow_and_task() {
    let nodes = json!([
        {"id": "t1", "kind": "tool", "slug": "uniswap", "chains": ["base"], "x": 0, "y": 0},
        {"id": "n1", "kind": "note", "text": "Start here", "x": 40, "y": 40}
    ]);
    let edges = json!([
        {
            "id": "e1",
            "fromId": "t1",
            "toId": "n1",
            "style": "arrow",
            "color": "#E76F00"
        }
    ]);
    let mut tool_meta = HashMap::new();
    tool_meta.insert(
        "uniswap".into(),
        ToolExportMeta {
            name: "Uniswap".into(),
            install_risk_level: "low".into(),
        },
    );

    let markdown = build_agent_export_markdown("My Stack", &nodes, &edges, &tool_meta, "cursor");

    assert!(markdown.starts_with("# My Stack\n"));
    assert!(markdown.contains("get_install_guide"));
    assert!(markdown.contains("### Uniswap"));
    assert!(markdown.contains("- Slug: `uniswap`"));
    assert!(markdown.contains("- Chains: base"));
    assert!(markdown.contains("- Install risk: low"));
    assert!(markdown.contains(&format!("{SITE_ORIGIN}/tools/uniswap")));
    assert!(markdown.contains("## Notes"));
    assert!(markdown.contains("- Start here"));
    assert!(markdown.contains("## Flow"));
    assert!(markdown.contains("uniswap → note: Start here"));
    assert!(markdown.contains("## Your task"));
    assert!(markdown.contains("do not install critical-risk tools"));
    assert!(markdown.contains("export PNG separately from the editor Share dock"));
}

#[test]
fn build_order_section_lists_stepped_nodes_sorted() {
    let nodes = parse_export_nodes(&json!([
        {"id": "t2", "kind": "tool", "slug": "beta", "x": 0, "y": 0, "step": 2},
        {"id": "n1", "kind": "note", "text": "check", "x": 0, "y": 0, "step": 1},
        {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0, "step": 3}
    ]));

    let order = build_order_section(&nodes);

    assert_eq!(
        order,
        "- 1. note: check (note)\n- 2. beta (tool)\n- 3. alpha (tool)"
    );
}

#[test]
fn build_order_section_tiebreaks_duplicate_steps_by_node_id() {
    let nodes = parse_export_nodes(&json!([
        {"id": "t2", "kind": "tool", "slug": "beta", "x": 0, "y": 0, "step": 1},
        {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0, "step": 1}
    ]));

    let order = build_order_section(&nodes);

    assert_eq!(order, "- 1. alpha (tool)\n- 1. beta (tool)");
}

#[test]
fn build_order_section_returns_empty_without_steps() {
    let nodes = parse_export_nodes(&json!([
        {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0}
    ]));

    assert!(build_order_section(&nodes).is_empty());
}

#[test]
fn build_order_section_supports_multi_step_nodes() {
    let nodes = parse_export_nodes(&json!([
        {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0, "steps": [1, 7]},
        {"id": "t2", "kind": "tool", "slug": "beta", "x": 0, "y": 0, "steps": [3]}
    ]));

    let order = build_order_section(&nodes);

    assert_eq!(
        order,
        "- 1. alpha (tool)\n- 3. beta (tool)\n- 7. alpha (tool)"
    );
}

#[test]
fn build_agent_export_markdown_includes_step_badges_in_tool_heading() {
    let nodes = json!([
        {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0, "steps": [1, 7]}
    ]);
    let markdown =
        build_agent_export_markdown("Stack", &nodes, &json!([]), &HashMap::new(), "generic");

    assert!(markdown.contains("### alpha #1 #7"));
    assert!(markdown.contains("platform: \"generic\""));
}

#[test]
fn build_agent_export_markdown_includes_order_section_after_notes() {
    let nodes = json!([
        {"id": "t1", "kind": "tool", "slug": "alpha", "x": 0, "y": 0, "step": 1},
        {"id": "n1", "kind": "note", "text": "memo", "x": 0, "y": 0}
    ]);
    let markdown =
        build_agent_export_markdown("Stack", &nodes, &json!([]), &HashMap::new(), "cursor");

    let notes_pos = markdown.find("## Notes").unwrap();
    let order_pos = markdown.find("## Order").unwrap();
    let flow_pos = markdown.find("## Flow").unwrap();
    assert!(notes_pos < order_pos);
    assert!(order_pos < flow_pos);
    assert!(markdown.contains("- 1. alpha (tool)"));
    assert!(markdown.contains("treat it as the owner's step sequence"));
}

#[test]
fn build_agent_export_markdown_sorts_tools_by_step_then_canvas_order() {
    let nodes = json!([
        {"id": "t1", "kind": "tool", "slug": "first", "x": 0, "y": 0},
        {"id": "t2", "kind": "tool", "slug": "second", "x": 10, "y": 0, "step": 2},
        {"id": "t3", "kind": "tool", "slug": "third", "x": 20, "y": 0, "step": 1},
        {"id": "t4", "kind": "tool", "slug": "fourth", "x": 30, "y": 0}
    ]);
    let markdown =
        build_agent_export_markdown("Stack", &nodes, &json!([]), &HashMap::new(), "cursor");

    let third_pos = markdown.find("### third").unwrap();
    let second_pos = markdown.find("### second").unwrap();
    let first_pos = markdown.find("### first").unwrap();
    let fourth_pos = markdown.find("### fourth").unwrap();
    assert!(third_pos < second_pos);
    assert!(second_pos < first_pos);
    assert!(first_pos < fourth_pos);
}

#[test]
fn build_flow_section_annotates_dashed_edges() {
    let nodes = parse_export_nodes(&json!([
        {"id": "a", "kind": "tool", "slug": "alpha", "x": 0, "y": 0},
        {"id": "b", "kind": "tool", "slug": "beta", "x": 40, "y": 0},
        {"id": "c", "kind": "tool", "slug": "gamma", "x": 80, "y": 0}
    ]));
    let edges = json!([
        {"id": "e1", "fromId": "a", "toId": "b", "style": "arrow", "color": "#1A1A1A",
         "dashed": true, "label": "optional"},
        {"id": "e2", "fromId": "b", "toId": "c", "style": "arrow", "color": "#1A1A1A",
         "dashed": true}
    ]);

    let flow = build_flow_section(&nodes, &edges);

    assert!(flow.contains("alpha →(optional, dashed) beta"));
    assert!(flow.contains("beta -[dashed]→ gamma"));
}

#[test]
fn build_flow_section_lists_edges_when_cycle_detected() {
    let nodes = parse_export_nodes(&json!([
        {"id": "a", "kind": "tool", "slug": "alpha", "x": 0, "y": 0},
        {"id": "b", "kind": "tool", "slug": "beta", "x": 40, "y": 40}
    ]));
    let edges = json!([
        {"id": "e1", "fromId": "a", "toId": "b", "style": "solid", "color": "#1A1A1A"},
        {"id": "e2", "fromId": "b", "toId": "a", "style": "solid", "color": "#1A1A1A"}
    ]);

    let flow = build_flow_section(&nodes, &edges);

    assert!(flow.contains("alpha → beta"));
    assert!(flow.contains("beta → alpha"));
}

#[test]
fn collect_tool_slugs_deduplicates_in_order() {
    let nodes = json!([
        {"id": "t1", "kind": "tool", "slug": "foo", "x": 0, "y": 0},
        {"id": "t2", "kind": "tool", "slug": "bar", "x": 10, "y": 10},
        {"id": "t3", "kind": "tool", "slug": "foo", "x": 20, "y": 20}
    ]);

    assert_eq!(collect_tool_slugs(&nodes), vec!["foo", "bar"]);
}

#[test]
fn validate_nodes_clamps_size_and_step() {
    let nodes = json!([
        {"id": "t1", "kind": "tool", "slug": "foo", "x": 0, "y": 0,
         "w": 9000, "h": 10, "step": 250},
        {"id": "n1", "kind": "note", "text": "", "x": 8, "y": 8, "w": 300, "h": 200},
        {"id": "c1", "kind": "chain", "chainId": "base", "x": 0, "y": 0, "w": 400, "step": 2}
    ]);
    let result = validate_nodes(&nodes).unwrap();
    let arr = result.as_array().unwrap();
    // Tool: width clamped down to max, height clamped up to min, step migrated to steps array capped.
    assert_eq!(arr[0]["w"], json!(NODE_MAX_W));
    assert_eq!(arr[0]["h"], json!(NODE_MIN_H));
    assert_eq!(arr[0]["steps"], json!([NODE_MAX_STEP]));
    // Note keeps in-range size.
    assert_eq!(arr[1]["w"], json!(300));
    assert_eq!(arr[1]["h"], json!(200));
    // Chain never carries a size, but does carry steps (migrated from legacy step).
    assert!(arr[2].get("w").is_none());
    assert_eq!(arr[2]["steps"], json!([2]));
}

#[test]
fn validate_edges_preserves_dashed_and_label() {
    let nodes = json!([
        {"id": "a", "kind": "tool", "slug": "foo", "x": 0, "y": 0},
        {"id": "b", "kind": "note", "text": "", "x": 40, "y": 40}
    ]);
    let node_ids = node_ids_from_value(&validate_nodes(&nodes).unwrap()).unwrap();
    let edges = json!([
        {"id": "e1", "fromId": "a", "toId": "b", "style": "arrow",
         "color": "#E76F00", "dashed": true, "label": "  swap to Base  "}
    ]);
    let result = validate_edges(&edges, &node_ids).unwrap();
    assert_eq!(result[0]["dashed"], json!(true));
    assert_eq!(result[0]["label"], json!("swap to Base"));
}

#[test]
fn validate_edges_omits_falsey_dashed_and_empty_label() {
    let nodes = json!([
        {"id": "a", "kind": "tool", "slug": "foo", "x": 0, "y": 0},
        {"id": "b", "kind": "note", "text": "", "x": 40, "y": 40}
    ]);
    let node_ids = node_ids_from_value(&validate_nodes(&nodes).unwrap()).unwrap();
    let edges = json!([
        {"id": "e1", "fromId": "a", "toId": "b", "style": "solid",
         "color": "#1A1A1A", "dashed": false, "label": "   "}
    ]);
    let result = validate_edges(&edges, &node_ids).unwrap();
    assert!(result[0].get("dashed").is_none());
    assert!(result[0].get("label").is_none());
}

#[test]
fn build_flow_section_splits_at_branch_points() {
    let nodes = parse_export_nodes(&json!([
        {"id": "hub", "kind": "tool", "slug": "gateway", "x": 0, "y": 0},
        {"id": "base", "kind": "chain", "chainId": "base", "x": 40, "y": 0},
        {"id": "bnb", "kind": "chain", "chainId": "bsc", "x": 40, "y": 40}
    ]));
    let edges = json!([
        {"id": "e1", "fromId": "hub", "toId": "base", "style": "arrow", "color": "#1A1A1A"},
        {"id": "e2", "fromId": "hub", "toId": "bnb", "style": "arrow", "color": "#1A1A1A",
         "label": "swap"}
    ]);

    let flow = build_flow_section(&nodes, &edges);
    let lines: Vec<&str> = flow.lines().collect();

    // Branch point produces one line per outgoing branch, not a single chain.
    assert_eq!(lines.len(), 2);
    assert!(flow.contains("gateway → chain: base"));
    assert!(flow.contains("gateway →(swap) chain: bsc"));
}

#[test]
fn build_flow_section_keeps_linear_path_on_one_line() {
    let nodes = parse_export_nodes(&json!([
        {"id": "a", "kind": "tool", "slug": "alpha", "x": 0, "y": 0},
        {"id": "b", "kind": "tool", "slug": "beta", "x": 40, "y": 0},
        {"id": "c", "kind": "tool", "slug": "gamma", "x": 80, "y": 0}
    ]));
    let edges = json!([
        {"id": "e1", "fromId": "a", "toId": "b", "style": "arrow", "color": "#1A1A1A"},
        {"id": "e2", "fromId": "b", "toId": "c", "style": "arrow", "color": "#1A1A1A"}
    ]);

    let flow = build_flow_section(&nodes, &edges);

    assert_eq!(flow, "- alpha → beta → gamma");
}
