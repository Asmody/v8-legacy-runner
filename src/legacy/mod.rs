//! Дельта форка `v8-legacy-runner` поверх апстрима `IngvarConsulting/v8-runner-rust`.
//!
//! Правила дельты
//! (спека `docs/superpowers/specs/2026-11-06-v8-legacy-runner-overlay-design.md`):
//! всё знание дельты живёт здесь; в файлах апстрима — только швы из 1–8 строк: вызовы
//! `crate::legacy::*`, значения констант, тесты. Никакой логики inline.
//!
//! Таблица швов — канонический перечень «что наложить при обновлении из апстрима»:
//!
//! | # | Файл апстрима | Шов |
//! |---|---|---|
//! | 0 | `main.rs` | `mod legacy;` |
//! | 1 | `platform/generation.rs` | без изменений (апстрим-вид) |
//! | 2 | `platform/designer.rs::config_generation_id` | precheck `probe_precheck` + плечо `TimedOut` → `warn!` + `Ok(None)` |
//! | 3 | `platform/ibcmd.rs::config_generation_id` | то же, что #2 |
//! | 4 | `use_cases/generation_reader.rs` (оба места `process_policy`) | `Some(crate::legacy::generation::PROBE_CAP)` |
//! | 5 | `platform/locator.rs::MINIMUM_PLATFORM` | 8.3.24 → 8.3.12, комментарий ведёт в `legacy::floor` |
//! | 6 | `use_cases/exchange_guard.rs::compare` | ветка «ответа нет» → `warn_unchecked` |
//! | 7 | тесты минимума: `locator.rs`, `use_cases/result.rs`, `tests/cli_minimum_platform.rs` | фиксуры 8.3.23 → 8.3.11, 8.3.24 → 8.3.12 |

pub mod floor;
pub mod generation;
