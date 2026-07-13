//! Blueprint agent markdown export.
use std::collections::{HashMap, HashSet};

use axum::http::HeaderMap;
use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::Value;
use uuid::Uuid;

use crate::config::SITE_ORIGIN;
use crate::AppState;

use super::super::auth::require_user_from;
use super::super::error::ApiError;
use super::access::fetch_owned_blueprint;
use super::types::*;
use super::validate::validate_title;

pub(crate) fn parse_export_nodes(nodes: &Value) -> Vec<ExportNode> {
    let Some(arr) = nodes.as_array() else {
        return Vec::new();
    };

    arr.iter()
        .filter_map(|item| {
            let id = item.get("id")?.as_str()?.trim().to_string();
            if id.is_empty() {
                return None;
            }
            let kind = item.get("kind")?.as_str()?.to_string();
            let slug = item
                .get("slug")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let chain_id = item
                .get("chainId")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let text = item
                .get("text")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let chains = item
                .get("chains")
                .and_then(|v| v.as_array())
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|chain| chain.as_str().map(str::trim))
                        .filter(|chain| !chain.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let steps: Vec<i32> = item
                .get("steps")
                .and_then(|v| v.as_array())
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|s| s.as_i64().map(|v| v as i32))
                        .filter(|s| *s >= 1)
                        .map(|s| s.min(NODE_MAX_STEP))
                        .take(NODE_MAX_STEPS_PER_NODE)
                        .collect()
                })
                .unwrap_or_default();
            // Fall back to legacy `step` field
            let mut steps = if steps.is_empty() {
                item.get("step")
                    .and_then(|v| v.as_i64())
                    .map(|v| vec![v.min(NODE_MAX_STEP as i64) as i32])
                    .unwrap_or_default()
            } else {
                steps
            };
            // Dedupe and sort so steps.first() is always the minimum
            steps.sort();
            steps.dedup();
            steps.truncate(NODE_MAX_STEPS_PER_NODE);

            Some(ExportNode {
                id,
                kind,
                slug,
                chain_id,
                text,
                chains,
                steps,
            })
        })
        .collect()
}

pub(crate) fn collect_tool_slugs(nodes: &Value) -> Vec<String> {
    let mut slugs = Vec::new();
    let mut seen = HashSet::new();
    for node in parse_export_nodes(nodes) {
        if node.kind == "tool" {
            if let Some(slug) = node.slug {
                if seen.insert(slug.clone()) {
                    slugs.push(slug);
                }
            }
        }
    }
    slugs
}

pub(crate) fn node_flow_label(node: &ExportNode) -> String {
    match node.kind.as_str() {
        "tool" => node.slug.clone().unwrap_or_else(|| node.id.clone()),
        "note" => {
            let text = node.text.as_deref().unwrap_or("").trim();
            if text.is_empty() {
                "note".into()
            } else if text.chars().count() > 48 {
                let truncated: String = text.chars().take(48).collect();
                format!("note: {truncated}…")
            } else {
                format!("note: {text}")
            }
        }
        "chain" => format!("chain: {}", node.chain_id.as_deref().unwrap_or("unknown")),
        _ => node.id.clone(),
    }
}

struct FlowEdge {
    from: String,
    to: String,
    label: Option<String>,
    dashed: bool,
}

pub(crate) fn flow_label_for(node_map: &HashMap<&str, &ExportNode>, id: &str) -> String {
    node_map
        .get(id)
        .map(|node| node_flow_label(node))
        .unwrap_or_else(|| id.to_string())
}

/// Follow a maximal simple path from `start_idx`, stopping at branch/merge
/// points (nodes whose in- or out-degree is not exactly 1) or a revisited edge.
pub(crate) fn walk_flow_segment(
    start_idx: usize,
    flow_edges: &[FlowEdge],
    out_edges: &HashMap<String, Vec<usize>>,
    in_deg: &HashMap<String, usize>,
    out_deg: &HashMap<String, usize>,
    node_map: &HashMap<&str, &ExportNode>,
    visited: &mut [bool],
) -> String {
    let mut line = flow_label_for(node_map, &flow_edges[start_idx].from);
    let mut cur = start_idx;
    loop {
        visited[cur] = true;
        let edge = &flow_edges[cur];
        if edge.dashed {
            match &edge.label {
                Some(label) => line.push_str(&format!(" →({label}, dashed) ")),
                None => line.push_str(" -[dashed]→ "),
            }
        } else {
            match &edge.label {
                Some(label) => line.push_str(&format!(" →({label}) ")),
                None => line.push_str(" → "),
            }
        }
        line.push_str(&flow_label_for(node_map, &edge.to));

        let internal = in_deg.get(&edge.to).copied().unwrap_or(0) == 1
            && out_deg.get(&edge.to).copied().unwrap_or(0) == 1;
        if internal {
            if let Some(next) = out_edges
                .get(&edge.to)
                .and_then(|list| list.first().copied())
            {
                if !visited[next] {
                    cur = next;
                    continue;
                }
            }
        }
        break;
    }
    line
}

pub(crate) fn build_flow_section(nodes: &[ExportNode], edges: &Value) -> String {
    let node_map: HashMap<&str, &ExportNode> =
        nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let edge_arr = edges.as_array().cloned().unwrap_or_default();

    // Keep only edges whose endpoints both resolve, preserving insertion order.
    let mut flow_edges: Vec<FlowEdge> = Vec::new();
    for edge in edge_arr {
        let from = edge
            .get("fromId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let to = edge
            .get("toId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if from.is_empty() || to.is_empty() {
            continue;
        }
        if !node_map.contains_key(from) || !node_map.contains_key(to) {
            continue;
        }
        let label = edge
            .get("label")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let dashed = edge
            .get("dashed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        flow_edges.push(FlowEdge {
            from: from.to_string(),
            to: to.to_string(),
            label,
            dashed,
        });
    }

    if flow_edges.is_empty() {
        return "(no flow edges defined)".into();
    }

    let mut in_deg: HashMap<String, usize> = HashMap::new();
    let mut out_deg: HashMap<String, usize> = HashMap::new();
    let mut out_edges: HashMap<String, Vec<usize>> = HashMap::new();
    for node in nodes {
        in_deg.entry(node.id.clone()).or_insert(0);
        out_deg.entry(node.id.clone()).or_insert(0);
    }
    for (idx, edge) in flow_edges.iter().enumerate() {
        *out_deg.entry(edge.from.clone()).or_insert(0) += 1;
        *in_deg.entry(edge.to.clone()).or_insert(0) += 1;
        in_deg.entry(edge.from.clone()).or_insert(0);
        out_deg.entry(edge.to.clone()).or_insert(0);
        out_edges.entry(edge.from.clone()).or_default().push(idx);
    }

    // Deterministic output: sort out-edges by target label, then id, then index.
    for indices in out_edges.values_mut() {
        indices.sort_by(|&a, &b| {
            flow_label_for(&node_map, &flow_edges[a].to)
                .cmp(&flow_label_for(&node_map, &flow_edges[b].to))
                .then_with(|| flow_edges[a].to.cmp(&flow_edges[b].to))
                .then_with(|| a.cmp(&b))
        });
    }

    let mut visited = vec![false; flow_edges.len()];
    let mut lines: Vec<String> = Vec::new();

    // 1. Segments that begin at a junction (source/sink/branch/merge).
    let mut junctions: Vec<String> = out_edges.keys().cloned().collect();
    junctions.sort_by(|a, b| {
        flow_label_for(&node_map, a)
            .cmp(&flow_label_for(&node_map, b))
            .then_with(|| a.cmp(b))
    });
    for from in &junctions {
        let is_junction = in_deg.get(from).copied().unwrap_or(0) != 1
            || out_deg.get(from).copied().unwrap_or(0) != 1;
        if !is_junction {
            continue;
        }
        let idx_list = out_edges.get(from).cloned().unwrap_or_default();
        for idx in idx_list {
            if !visited[idx] {
                lines.push(walk_flow_segment(
                    idx,
                    &flow_edges,
                    &out_edges,
                    &in_deg,
                    &out_deg,
                    &node_map,
                    &mut visited,
                ));
            }
        }
    }

    // 2. Remaining edges belong to pure cycles with no junction — emit them too.
    for idx in 0..flow_edges.len() {
        if !visited[idx] {
            lines.push(walk_flow_segment(
                idx,
                &flow_edges,
                &out_edges,
                &in_deg,
                &out_deg,
                &node_map,
                &mut visited,
            ));
        }
    }

    // 3. Nodes touching no edge, listed on their own for completeness.
    let touched: HashSet<&str> = flow_edges
        .iter()
        .flat_map(|edge| [edge.from.as_str(), edge.to.as_str()])
        .collect();
    let mut orphans: Vec<String> = nodes
        .iter()
        .filter(|node| !touched.contains(node.id.as_str()))
        .map(node_flow_label)
        .collect();
    orphans.sort();
    lines.extend(orphans);

    if lines.is_empty() {
        "(no flow edges defined)".into()
    } else {
        lines
            .iter()
            .map(|line| format!("- {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(crate) fn build_order_section(nodes: &[ExportNode]) -> String {
    // Expand each node's steps into (node, step) pairs, then sort globally.
    let mut stepped: Vec<(&ExportNode, i32)> = Vec::new();
    for node in nodes {
        for &step in &node.steps {
            stepped.push((node, step));
        }
    }
    if stepped.is_empty() {
        return String::new();
    }
    stepped.sort_by(|(a, step_a), (b, step_b)| step_a.cmp(step_b).then_with(|| a.id.cmp(&b.id)));
    stepped
        .into_iter()
        .map(|(node, step)| format!("- {step}. {} ({})", node_flow_label(node), node.kind))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn build_agent_export_markdown(
    title: &str,
    nodes: &Value,
    edges: &Value,
    tool_meta: &HashMap<String, ToolExportMeta>,
    platform: &str,
) -> String {
    let export_nodes = parse_export_nodes(nodes);
    let mut markdown = format!("# {title}\n\n");
    markdown.push_str(&format!(
        "Read the attached blueprint image together with this prompt. \
For each tool below, call OnchainAI MCP `get_install_guide` (platform: {platform}) \
before installing.\n\n",
    ));

    markdown.push_str("## Tools\n\n");
    let mut tool_nodes: Vec<(usize, &ExportNode)> = export_nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.kind == "tool")
        .collect();
    tool_nodes.sort_by(|(idx_a, a), (idx_b, b)| {
        let a_min = a.steps.first().copied();
        let b_min = b.steps.first().copied();
        match (a_min, b_min) {
            (Some(step_a), Some(step_b)) => step_a.cmp(&step_b).then(idx_a.cmp(idx_b)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => idx_a.cmp(idx_b),
        }
    });
    if tool_nodes.is_empty() {
        markdown.push_str("(none)\n\n");
    } else {
        for (_, node) in tool_nodes {
            let slug = node.slug.as_deref().unwrap_or("unknown");
            let display_name = tool_meta
                .get(slug)
                .map(|meta| meta.name.as_str())
                .unwrap_or(slug);
            let chains = if node.chains.is_empty() {
                "none specified".to_string()
            } else {
                node.chains.join(", ")
            };
            let step_badges: String = if node.steps.is_empty() {
                String::new()
            } else {
                let badges: Vec<String> = node.steps.iter().map(|s| format!("#{s}")).collect();
                format!(" {}", badges.join(" "))
            };
            markdown.push_str(&format!("### {display_name}{step_badges}\n"));
            markdown.push_str(&format!("- Slug: `{slug}`\n"));
            markdown.push_str(&format!("- Chains: {chains}\n"));
            if let Some(meta) = tool_meta.get(slug) {
                markdown.push_str(&format!("- Install risk: {}\n", meta.install_risk_level));
            }
            markdown.push_str(&format!("- Page: {SITE_ORIGIN}/tools/{slug}\n"));
            markdown.push_str(&format!(
                "- MCP: `get_install_guide({{ slug: \"{slug}\", platform: \"{platform}\" }})`\n\n"
            ));
        }
    }

    markdown.push_str("## Notes\n\n");
    let note_texts: Vec<String> = export_nodes
        .iter()
        .filter(|node| node.kind == "note")
        .filter_map(|node| node.text.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .collect();
    if note_texts.is_empty() {
        markdown.push_str("(none)\n\n");
    } else {
        for text in note_texts {
            markdown.push_str(&format!("- {text}\n"));
        }
        markdown.push('\n');
    }

    let order_section = build_order_section(&export_nodes);
    if !order_section.is_empty() {
        markdown.push_str("## Order\n\n");
        markdown.push_str(&order_section);
        markdown.push_str("\n\n");
    }

    markdown.push_str("## Flow\n\n");
    markdown.push_str(&build_flow_section(&export_nodes, edges));
    markdown.push_str("\n\n");
    markdown.push_str(&AGENT_EXPORT_TASK_TEMPLATE.replace("{platform}", platform));
    markdown
}

pub(crate) async fn fetch_tool_export_meta(
    state: &AppState,
    slugs: &[String],
) -> Result<HashMap<String, ToolExportMeta>, ApiError> {
    if slugs.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    struct ToolExportRow {
        slug: String,
        name: String,
        install_risk_level: String,
    }

    let rows = sqlx::query_as::<_, ToolExportRow>(
        r#"
        SELECT slug, name, install_risk_level
        FROM tools
        WHERE slug = ANY($1)
          AND approval_status = 'approved'
        "#,
    )
    .bind(slugs)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| db_internal("tool lookup", e))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.slug,
                ToolExportMeta {
                    name: row.name,
                    install_risk_level: row.install_risk_level,
                },
            )
        })
        .collect())
}

pub(crate) async fn agent_export_blueprint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<AgentExportResponse>, ApiError> {
    let user = require_user_from(&state, &headers).await?;
    let row = fetch_owned_blueprint(&state, id, user.id).await?;
    let slugs = collect_tool_slugs(&row.nodes);
    let tool_meta = fetch_tool_export_meta(&state, &slugs).await?;

    // Default platform is generic; cursor/claude also accepted.
    let platform = headers
        .get("x-blueprint-platform")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("generic");
    let platform = match platform {
        "cursor" => "cursor",
        "claude" => "claude",
        _ => "generic",
    };

    let markdown =
        build_agent_export_markdown(&row.title, &row.nodes, &row.edges, &tool_meta, platform);

    Ok(Json(AgentExportResponse {
        title: row.title,
        markdown,
        slugs,
        filename: AGENT_EXPORT_FILENAME.into(),
    }))
}
