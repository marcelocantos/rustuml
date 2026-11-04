# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
import json
from pathlib import Path
base=Path('docs/parity-reviews')
items=[
('sequence-registered-command-rejection',
 ['sequencediagram.SequenceDiagramFactory#initCommandsList','command.CommonCommands#addCommonCommands1','command.PSystemCommandFactory#getCandidate','command.PSystemCommandFactory#executeFewLines','command.SingleLineCommand2#isValid','command.CommandMultilines2#isValid','command.CommandMultilines#isValid','regex.Pattern2#transform','regex.RegexConcat#match'],
 ['crates/rustuml-parser/src/parse/sequence.rs::parse_sequence','crates/rustuml-parser/src/parse/sequence.rs::SeqParser::parse_line'],
 'The ordered registered command language decides whether each complete command is syntactically admissible. No candidate produces a syntax error; a registered command whose execution is not implemented is separately observable residue, not a syntax error and not silent support.',
 'Rust handler fallthrough sets unconsumed but parse_sequence returns a diagram. Several permissive prefix handlers also consume lines outside Java command grammar. Java PSystemCommandFactory rejects when getCandidate returns null, after complete multiline and registered-order matching.',
 'Record the complete registered grammar and map handlers. Add a source-derived admission/consumption layer in sequence.rs, including multiline boundaries, Pattern2 classes and FoxSignature preconditions. Keep recognized unsupported commands nonfatal with class/source diagnostics; propagate absence of any command as ParseError. Validate both previously consumed and unconsumed lines, without changing global factory dispatch.',
 ['command family and registration overlap','valid/invalid separators and command suffixes','single/multiline boundaries and supported/unimplemented handlers']),
('sequence-command-color-execution',
 ['command.SingleLineCommand2#execute','command.CommandMultilines2#execute','klimt.color.ColorParser#getRegex','klimt.color.ColorParser#getColor','klimt.color.Colors#Colors','klimt.color.HColorSet#getColor','klimt.color.HColorSet#parseSimpleColor','command.note.sequence.FactorySequenceNoteCommand#executeInternal'],
 ['crates/rustuml-parser/src/parse/sequence.rs::SeqParser::try_note','crates/rustuml-parser/src/parse/sequence.rs::valid_return_color'],
 'After syntactic command selection, captured colors are interpreted using the command color grammar and HColorSet; an unknown color yields a command execution error, including when a richer ColorParser alternative fails and the simple alternative captures a shorter name.',
 'Rust note patterns approximate color with a delimiter class and store arbitrary names without validating execution. In #back:LightBlue: cargo, Java PART2 cannot end before colon, the simple alternative captures #back, and Colors/HColorSet throws; Rust currently emits a successful SVG.',
 'Use registered command captures and source-derived ColorParser alternative/termination semantics for sequence color execution. Reuse the existing source-derived color name domain, extend validation only according to HColorSet/Colors, propagate badColor through ParseError rather than accepting a fallback paint color. Preserve valid simple/composite/gradient/transparent colors and distinguish unsupported paint residue.',
 ['note shape/attachment and engine','simple/composite color grammar, delimiter placement and unknown names']),
('sequence-footbox-command-consumption',
 ['sequencediagram.command.CommandFootbox#HIDE_SHOW_FOOTBOX','sequencediagram.command.CommandFootbox#executeArg','regex.RegexLeaf#spaceZeroOrMore','regex.Pattern2#transform'],
 ['crates/rustuml-parser/src/parse/sequence.rs::SeqParser::try_hide'],
 'Footbox visibility consumes the registered optional hide/show prefix and zero-or-more Pattern2 whitespace; there is no mandatory FoxSignature whitespace bit.',
 'Rust uses literal hide footbox/show footbox checks, missing Java-valid VT and zero separator forms, leaving the default visibility.',
 'Parse the complete CommandFootbox expression in the sequence visibility handler and apply the captured action; keep the legacy footbox command independently inventoried.',
 ['hide/show and separator placement','legacy/Teoz and participant topology']),
('sequence-local-attribute-serialization',
 ['klimt.drawing.svg.SvgGraphics#createXml','klimt.drawing.svg.SvgGraphics#getTransformer','xml.XmlFactories#newTransformer','OpenJDK21.0.5.ToStream#writeAttrString','OpenJDK21.0.5.ToStream#accumDefaultEscape','OpenJDK21.0.5.CharInfo'],
 ['crates/rustuml-render/src/sequence.rs::escape_xml'],
 'Sequence-owned SVG attribute serialization matches pinned us-ascii XML output: escape XML metacharacters, encode LF/CR/TAB and non-ASCII Unicode scalars as decimal character references, and retain literal apostrophes.',
 'The local helper only escapes XML metacharacters and two guillemets. Other Unicode and attribute whitespace differ from Java serialized bytes and attribute parsing semantics. Shared maker trace identifies exact JAXP behavior; this branch must implement its own owned helper without importing concurrent shared code.',
 'Iterate Unicode scalars in the local attribute helper with explicit XML mappings and decimal references. Audit every caller to distinguish attributes and text content; preserve text-node behavior through its appropriate path. Invalid other C0 controls remain T14.20 exporter residue, not simulated crashes. Preserve the received source trace and hashes in new evidence.',
 ['ASCII metacharacters/attribute whitespace versus non-ASCII and non-BMP','participant data attributes versus hyperlink attributes']),
('sequence-resume-citation-correction',
 ['sequencediagram.command.CommandAutonumberResume#getConcat'],
 ['crates/rustuml-parser/src/parse/sequence.rs::SeqParser::try_autonumber'],
 'Adjacent source provenance names an existing pinned Java method; correcting the citation does not retroactively change prior accounts or verdicts.',
 'The current comment and earlier immutable account name getRegexConcat, but CommandAutonumberResume declares private static RegexConcat getConcat.',
 'Append this correction as new evidence and change only the current adjacent method citation, preserving earlier accounts and reviews byte-for-byte.',
 ['source declaration verification','production comment and immutable historical account comparison'])]
for name,java,rust,invariant,cause,change,axes in items:
 p=base/name;p.mkdir(exist_ok=False)
 obj=dict(schema_version=1,mechanism=name,java_revision='71806a23780b04a5ccde2f8ceb5121edad5eb711',java_locations=java,rust_locations=rust,invariant=invariant,causal_divergence=cause,planned_change=change,perturbation_axes=axes,baseline_evidence='dc438fd0cd0f5edd44816db81f0d765bf48ce4e6',baseline_production='b4849a5008ff28c360fefe0176ee6ad073c02fd7',rejection='0fb379a26ab64ead0744278935a0b6c64082ef35',status='preimplementation hypothesis; no acceptance claim')
 (p/'account.json').write_text(json.dumps(obj,indent=2)+'\n')
