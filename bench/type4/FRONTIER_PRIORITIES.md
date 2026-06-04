# Type-4 frontier priorities

This report is generated from the pinned benchmark repos by
`bench/type4/prioritize_frontier.py`. Scores combine real-code frequency,
repo/language spread, estimated implementation cost, soundness risk, scope,
and whether a frontier is already covered.

- repos scanned: 105
- files scanned: 59515
- max bytes per file: 512000

| rank | candidate | scope | status | score | matches | repos | languages | cost | risk |
|---:|---|---|---|---:|---:|---:|---:|---:|---:|
| 1 | `collection_empty_check` | all-language | open | 108.45 | 49377 | 92 | 7 | 2 | 2 |
| 2 | `string_prefix_suffix` | all-language | open | 90.11 | 6275 | 97 | 7 | 2 | 2 |
| 3 | `membership_contains` | multi-language | open | 66.89 | 77467 | 100 | 7 | 3 | 3 |
| 4 | `numeric_minmax_abs` | all-language | partially-covered | 65.09 | 7114 | 94 | 8 | 2 | 2 |
| 5 | `null_option_presence` | all-language | partially-covered | 52.69 | 160555 | 94 | 7 | 3 | 3 |
| 6 | `map_default_lookup` | multi-language | open | 45.85 | 51294 | 90 | 7 | 4 | 4 |
| 7 | `property_type_guard` | language-family | open | 5.01 | 436 | 19 | 2 | 2 | 3 |
| 8 | `own_property_guard` | language-family | covered-current | 0.61 | 797 | 23 | 2 | 1 | 3 |

## Recommended Order

1. `collection_empty_check`
   - why: Most languages expose both length comparison and named emptiness predicates.
   - evidence: 49377 matches across 92 repos and 7 languages (c, go, java, javascript, python, rust, typescript)
   - next probe: Generate `len(x) == 0` / `.is_empty()` / `.isEmpty()` / `.empty?` positives, with nonzero and wrong-collection negatives.
2. `string_prefix_suffix`
   - why: The API names differ by language but the strict predicate coordinate is simple.
   - evidence: 6275 matches across 97 repos and 7 languages (go, java, javascript, python, ruby, rust, typescript)
   - next probe: Lower case-sensitive starts-with/ends-with calls to prefix/suffix facts; keep regex, contains, and case-folding boundaries.
3. `membership_contains`
   - why: Common but semantically overloaded: substring, list membership, map key membership, and set membership must stay distinct.
   - evidence: 77467 matches across 100 repos and 7 languages (go, java, javascript, python, ruby, rust, typescript)
   - next probe: Start with static set/list membership only; keep substring, regex, and map-key boundaries separate.
4. `map_default_lookup`
   - why: Potentially high value, but absent-key semantics and mutation/effects vary heavily.
   - evidence: 51294 matches across 90 repos and 7 languages (go, java, javascript, python, ruby, rust, typescript)
   - next probe: Start with literal immutable maps and static keys; hard-negative missing-key/default-value changes.
5. `property_type_guard`
   - why: Very frequent in JS-family repos, but the scope is narrow and should wait behind broader axes.
   - evidence: 436 matches across 19 repos and 2 languages (javascript, typescript)
   - next probe: Generate `typeof obj.field === <type>` variants with dynamic-key and shadowing boundaries.

## Samples

### `collection_empty_check`
- `alacritty/alacritty/src/window_context.rs:413` (rust): if self.event_queue.is_empty() {
- `alacritty/alacritty/src/message_bar.rs:54` (rust): || (lines.is_empty() && num_cols >= button_len
- `alacritty/alacritty/src/message_bar.rs:91` (rust): if lines.len() > max_lines {
- `alacritty/alacritty/src/message_bar.rs:93` (rust): if TRUNCATED_MESSAGE.len() <= num_cols {
- `alacritty/alacritty/src/message_bar.rs:148` (rust): self.messages.is_empty() }

### `string_prefix_suffix`
- `alacritty/alacritty/src/polling/ipc.rs:197` (rust): .filter(|file| file.starts_with(&socket_prefix) && file.ends_with(".sock"))
- `alacritty/alacritty/src/polling/ipc.rs:197` (rust): .filter(|file| file.starts_with(&socket_prefix) && file.ends_with(".sock"))
- `alacritty/alacritty/src/config/mod.rs:215` (rust): if contents.starts_with('\u{FEFF}') {
- `alacritty/alacritty/src/config/bindings.rs:736` (rust): _ if keycode.starts_with("Dead") => {
- `alacritty/alacritty/src/display/color.rs:287` (rust): let chars = if s.starts_with("0x") && s.len() == 8 {

### `membership_contains`
- `alacritty/alacritty/src/window_context.rs:542` (rust): let origin_at_bottom = if terminal.mode().contains(TermMode::VI) {
- `alacritty/alacritty/src/message_bar.rs:188` (rust): self.messages.contains(message)
- `alacritty/alacritty/src/logging.rs:188` (rust): _ => ALLOWED_TARGETS.contains(&target) || extra_log_targets().iter().any(|t| t == target),
- `alacritty/alacritty/src/event.rs:720` (rust): let vi_mode = self.terminal.mode().contains(TermMode::VI);
- `alacritty/alacritty/src/event.rs:780` (rust): if self.terminal.mode().contains(TermMode::VI) && !self.search_active() {

### `numeric_minmax_abs`
- `alacritty/alacritty/src/window_context.rs:271` (rust): if (old_config.cursor.thickness() - self.config.cursor.thickness()).abs() > f32::EPSILON {
- `alacritty/alacritty/src/event.rs:1747` (rust): let font_delta = (delta.abs() / FONT_SIZE_STEP).floor() * FONT_SIZE_STEP * delta.signum();
- `alacritty/alacritty/src/renderer/rects.rs:98` (rust): Flags::UNDERCURL => (metrics.descent, metrics.descent.abs(), RectKind::Undercurl),
- `alacritty/alacritty/src/renderer/rects.rs:104` (rust): (metrics.descent, metrics.descent.abs(), RectKind::DottedUnderline)
- `alacritty/alacritty/src/renderer/rects.rs:136` (rust): thickness = thickness.max(1.);

### `null_option_presence`
- `alacritty/alacritty/build.rs:10` (rust): if let Some(commit_hash) = commit_hash() {
- `alacritty/alacritty/src/window_context.rs:90` (rust): let raw_window_handle = Some(window.raw_window_handle());
- `alacritty/alacritty/src/window_context.rs:93` (rust): let raw_window_handle = None;
- `alacritty/alacritty/src/window_context.rs:138` (rust): let tabbed = options.window_tabbing_id.is_some();
- `alacritty/alacritty/src/window_context.rs:154` (rust): renderer::platform::create_gl_context(&gl_display, gl_config, Some(raw_window_handle))?;

### `map_default_lookup`
- `alacritty/alacritty_terminal/src/tty/mod.rs:114` (rust): let first = terminfo.get(..1).unwrap_or_default();
- `antlr4/runtime-testsuite/test/org/antlr/v4/test/runtime/RuntimeTests.java:49` (java): File descriptorsDir = new File(Paths.get(RuntimeTestUtils.resourcePath.toString(), "org/antlr/v4/test/runtime/descriptors").toString());
- `antlr4/runtime-testsuite/test/org/antlr/v4/test/runtime/RuntimeTests.java:81` (java): RuntimeTestDescriptor[] descriptors = CustomDescriptors.descriptors.get(key);
- `antlr4/runtime-testsuite/test/org/antlr/v4/test/runtime/RuntimeTests.java:97` (java): RuntimeTestDescriptor[] descriptors = testDescriptors.get(group);
- `antlr4/runtime-testsuite/test/org/antlr/v4/test/runtime/RuntimeTests.java:110` (java): Path descriptorGroupPath = Paths.get(RuntimeTestUtils.resourcePath.toString(), "descriptors", group);

### `property_type_guard`
- `axios/tests/smoke/esm/tests/fetch.smoke.test.js:52` (javascript): isRequest && typeof input.clone === 'function'
- `axios/tests/smoke/cjs/tests/fetch.smoke.test.cjs:55` (javascript): isRequest && typeof input.clone === 'function'
- `axios/tests/unit/adapters/http.test.js:730` (javascript): const isZstdSupported = typeof zlib.createZstdDecompress === 'function' &&
- `axios/tests/unit/adapters/http.test.js:731` (javascript): typeof zlib.zstdCompress === 'function';
- `axios/tests/module/cjs/tests/helpers/fixture.cjs:19` (javascript): if (typeof fs.rmSync === 'function') {

### `own_property_guard`
- `axios/tests/unit/prototypePollution.test.js:71` (javascript): assert.strictEqual(result.hasOwnProperty('__proto__'), false);
- `axios/tests/unit/prototypePollution.test.js:78` (javascript): assert.strictEqual(result.hasOwnProperty('constructor'), false);
- `axios/tests/unit/prototypePollution.test.js:85` (javascript): assert.strictEqual(result.hasOwnProperty('prototype'), false);
- `axios/tests/unit/prototypePollution.test.js:101` (javascript): assert.strictEqual(result.headers.hasOwnProperty('__proto__'), false);
- `axios/tests/unit/prototypePollution.test.js:117` (javascript): assert.strictEqual(result.headers.hasOwnProperty('constructor'), false);
