// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Line-level delta reduction with protected PlantUML framing directives.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineReduction {
    pub source: String,
    pub original_line_count: usize,
    pub final_line_count: usize,
    pub attempts: usize,
    pub accepted: usize,
}

pub fn reduce_lines<E>(
    source: &str,
    mut preserves_failure: impl FnMut(&str) -> Result<bool, E>,
) -> Result<LineReduction, E> {
    let lines = source.split_inclusive('\n').collect::<Vec<_>>();
    let protected = lines
        .iter()
        .map(|line| {
            let line = line.trim_start();
            line.starts_with("@start") || line.starts_with("@end")
        })
        .collect::<Vec<_>>();
    let mut active = vec![true; lines.len()];
    let mut granularity = 2_usize;
    let mut attempts = 0_usize;
    let mut accepted = 0_usize;

    loop {
        let removable = active
            .iter()
            .enumerate()
            .filter_map(|(index, is_active)| (*is_active && !protected[index]).then_some(index))
            .collect::<Vec<_>>();
        if removable.is_empty() {
            break;
        }
        let chunk_size = removable.len().div_ceil(granularity);
        let mut reduced = false;
        for chunk in removable.chunks(chunk_size) {
            let mut candidate_active = active.clone();
            for index in chunk {
                candidate_active[*index] = false;
            }
            let candidate = assemble(&lines, &candidate_active);
            attempts += 1;
            if preserves_failure(&candidate)? {
                active = candidate_active;
                accepted += 1;
                granularity = granularity.saturating_sub(1).max(2);
                reduced = true;
                break;
            }
        }
        if reduced {
            continue;
        }
        if granularity >= removable.len() {
            break;
        }
        granularity = (granularity * 2).min(removable.len());
    }

    let reduced = assemble(&lines, &active);
    Ok(LineReduction {
        final_line_count: reduced.lines().count(),
        source: reduced,
        original_line_count: source.lines().count(),
        attempts,
        accepted,
    })
}

fn assemble(lines: &[&str], active: &[bool]) -> String {
    lines
        .iter()
        .zip(active)
        .filter_map(|(line, active)| active.then_some(*line))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::reduce_lines;

    #[test]
    fn protects_diagram_frame_and_finds_a_one_minimal_failure() {
        let source = "@startuml\nnoise one\nclass Trigger\nnoise two\n@enduml\n";
        let result = reduce_lines(source, |candidate| {
            Ok::<_, std::convert::Infallible>(candidate.contains("class Trigger"))
        })
        .unwrap();

        assert_eq!(result.source, "@startuml\nclass Trigger\n@enduml\n");
        assert_eq!(result.final_line_count, 3);
        assert!(result.attempts >= 2);
        assert!(result.accepted >= 1);
    }
}
