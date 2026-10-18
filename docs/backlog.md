# Application backlog

Proposed backlog: 10 epics and 39 stories (32 MVP, 7 expansion). No story is accepted. Stable keys link dependencies until GitHub issue numbers are available. G01 is the bootstrap review; all application implementation waits for its acceptance.

| Key | Story | Priority | Phase | Depends on |
|---|---|---|---|---|
| [EP01 #2](https://github.com/drluciano/doki/issues/2) | [EP01] Project governance, architecture and documentation | P0 | MVP |  |
| [EP02 #3](https://github.com/drluciano/doki/issues/3) | [EP02] Platform feasibility and native adapters | P0 | MVP |  |
| [EP03 #4](https://github.com/drluciano/doki/issues/4) | [EP03] Immutable checkpoints, history and retention | P0 | MVP |  |
| [EP04 #5](https://github.com/drluciano/doki/issues/5) | [EP04] Safe apply, recovery and independent export | P0 | MVP |  |
| [EP05 #6](https://github.com/drluciano/doki/issues/6) | [EP05] Emulator compatibility and game identity | P0 | MVP |  |
| [EP06 #7](https://github.com/drluciano/doki/issues/7) | [EP06] Security, pairing and private diagnostics | P0 | MVP |  |
| [EP07 #8](https://github.com/drluciano/doki/issues/8) | [EP07] Device exchange and save proposals | P0 | MVP |  |
| [EP08 #9](https://github.com/drluciano/doki/issues/9) | [EP08] Friendly experience, tutorial and accessible design | P0 | MVP |  |
| [EP09 #10](https://github.com/drluciano/doki/issues/10) | [EP09] Safety verification and Android/Windows beta | P0 | MVP |  |
| [EP10 #11](https://github.com/drluciano/doki/issues/11) | [EP10] Future platforms, transports and extensibility | P2 | Expansion |  |
| [F01 #12](https://github.com/drluciano/doki/issues/12) | [F01] Prove save-folder access on Windows and Android | P0 | MVP |  |
| [F02 #13](https://github.com/drluciano/doki/issues/13) | [F02] Define save-safety and recovery contracts | P0 | MVP | [F01 #12](https://github.com/drluciano/doki/issues/12) |
| [F03 #14](https://github.com/drluciano/doki/issues/14) | [F03] Define versioned modular architecture | P0 | MVP | [F02 #13](https://github.com/drluciano/doki/issues/13) |
| [G01 #1](https://github.com/drluciano/doki/issues/1) | [G01] Establish contribution and delivery workflow | P0 | MVP |  |
| [G02 #15](https://github.com/drluciano/doki/issues/15) | [G02] Provide reproducible development and CI | P0 | MVP | [F03 #14](https://github.com/drluciano/doki/issues/14) |
| [C01 #16](https://github.com/drluciano/doki/issues/16) | [C01] Create immutable local checkpoints | P0 | MVP | F02, F03, G02 |
| [C02 #17](https://github.com/drluciano/doki/issues/17) | [C02] Detect and preserve divergent progress | P0 | MVP | [C01 #16](https://github.com/drluciano/doki/issues/16) |
| [C03 #18](https://github.com/drluciano/doki/issues/18) | [C03] Apply a checkpoint with verified backup | P0 | MVP | C01, F01, P01, P02 |
| [C04 #19](https://github.com/drluciano/doki/issues/19) | [C04] Recover interrupted save installation | P0 | MVP | [C03 #18](https://github.com/drluciano/doki/issues/18) |
| [C05 #20](https://github.com/drluciano/doki/issues/20) | [C05] Restore and export historical checkpoints | P0 | MVP | [C04 #19](https://github.com/drluciano/doki/issues/19) |
| [C06 #21](https://github.com/drluciano/doki/issues/21) | [C06] Manage retention without surprising deletion | P1 | MVP | C05, C02 |
| [P01 #22](https://github.com/drluciano/doki/issues/22) | [P01] Implement Windows filesystem adapter | P0 | MVP | F01, F03 |
| [P02 #23](https://github.com/drluciano/doki/issues/23) | [P02] Implement Android document-access adapter | P0 | MVP | F01, F03 |
| [E01 #24](https://github.com/drluciano/doki/issues/24) | [E01] Define emulator profiles and save bundles | P0 | MVP | F01, C01 |
| [S01 #25](https://github.com/drluciano/doki/issues/25) | [S01] Design device trust and key recovery | P0 | MVP | [F03 #14](https://github.com/drluciano/doki/issues/14) |
| [S02 #26](https://github.com/drluciano/doki/issues/26) | [S02] Pair and revoke devices securely | P0 | MVP | S01, P01, P02 |
| [N01 #27](https://github.com/drluciano/doki/issues/27) | [N01] Specify resumable vault exchange protocol | P0 | MVP | F03, S01, C02 |
| [N02 #28](https://github.com/drluciano/doki/issues/28) | [N02] Implement the first self-hostable vault service | P0 | MVP | N01, S02 |
| [N03 #29](https://github.com/drluciano/doki/issues/29) | [N03] Receive and review save proposals | P0 | MVP | N02, C03, C02, E01, N05 |
| [N04 #30](https://github.com/drluciano/doki/issues/30) | [N04] Expose reliable transfer and background status | P1 | MVP | N03, P02 |
| [D01 #31](https://github.com/drluciano/doki/issues/31) | [D01] Create accessible Cocoon-inspired design system | P0 | MVP | [G01 #1](https://github.com/drluciano/doki/issues/1) |
| [D02 #32](https://github.com/drluciano/doki/issues/32) | [D02] Build library and game setup | P0 | MVP | D01, E01, C01, P01, P02 |
| [D03 #33](https://github.com/drluciano/doki/issues/33) | [D03] Build readable history and conflict review | P0 | MVP | D01, N03, C05 |
| [D04 #34](https://github.com/drluciano/doki/issues/34) | [D04] Teach safe sync with a disposable tutorial | P0 | MVP | D02, D03 |
| [Q01 #35](https://github.com/drluciano/doki/issues/35) | [Q01] Validate data safety through fault injection | P0 | MVP | C04, C06, N03, P01, P02 |
| [Q02 #36](https://github.com/drluciano/doki/issues/36) | [Q02] Provide private actionable diagnostics | P1 | MVP | F03, S01 |
| [R01 #37](https://github.com/drluciano/doki/issues/37) | [R01] Ship reviewable Android and Windows beta builds | P0 | MVP | Q01, D04, N04, Q02, G03 |
| [X01 #38](https://github.com/drluciano/doki/issues/38) | [X01] Add Linux platform support | P2 | Expansion | [R01 #37](https://github.com/drluciano/doki/issues/37) |
| [X02 #39](https://github.com/drluciano/doki/issues/39) | [X02] Add macOS platform support | P2 | Expansion | [R01 #37](https://github.com/drluciano/doki/issues/37) |
| [X03 #40](https://github.com/drluciano/doki/issues/40) | [X03] Research and implement supported iOS workflows | P2 | Expansion | [R01 #37](https://github.com/drluciano/doki/issues/37) |
| [X04 #41](https://github.com/drluciano/doki/issues/41) | [X04] Add alternate transport adapters | P2 | Expansion | R01, N01 |
| [X05 #42](https://github.com/drluciano/doki/issues/42) | [X05] Add optional format inspectors and semantic merge | P2 | Expansion | R01, E01, D03 |
| [X06 #43](https://github.com/drluciano/doki/issues/43) | [X06] Offer carefully scoped automation preferences | P2 | Expansion | R01, Q01 |
| [X07 #44](https://github.com/drluciano/doki/issues/44) | [X07] Expand scale, localization and extensibility | P2 | Expansion | [R01 #37](https://github.com/drluciano/doki/issues/37) |
| [C07 #45](https://github.com/drluciano/doki/issues/45) | [C07] Capture progress after a play session | P1 | MVP | C01, E01, P01, P02 |
| [E02 #46](https://github.com/drluciano/doki/issues/46) | [E02] Map the same game and save bundle across devices | P0 | MVP | E01, C02 |
| [N05 #47](https://github.com/drluciano/doki/issues/47) | [N05] Implement client checkpoint exchange and reconciliation | P0 | MVP | N02, C02, P01, P02, E02 |
| [D05 #48](https://github.com/drluciano/doki/issues/48) | [D05] Provide coherent app settings and device management | P1 | MVP | D01, S02, C06, C07, N04 |
| [G03 #49](https://github.com/drluciano/doki/issues/49) | [G03] Publish complete contributor and player documentation | P0 | MVP | G02, D04, Q02, D05 |

