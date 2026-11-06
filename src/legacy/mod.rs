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
//! | 4 | `use_cases/generation_reader.rs::read_generation` (место `process_policy`) | `Some(crate::legacy::generation::PROBE_CAP)`; в `read_unapplied` капы нет — это другой вопрос |
//! | 5 | `platform/locator.rs::MINIMUM_PLATFORM` | 8.3.24 → 8.3.12, комментарий ведёт в `legacy::floor` |
//! | 6 | `use_cases/exchange_guard.rs::compare` | ветка «ответа нет» → `warn_unchecked` |
//! | 7 | тесты минимума: `locator.rs`, `use_cases/result.rs`, `tests/cli_minimum_platform.rs` | фиксуры 8.3.23 → 8.3.11, 8.3.24 → 8.3.12 |
//! | 8 | `support/fs.rs::TOOL_NAME` / `is_known_tool_name` | имя `v8-legacy-runner`; признание своих lock'ов — `legacy::is_known_tool_name` |
//! | 9 | `cli/args.rs` `#[command(name)]` | `v8-legacy-runner` |

pub mod floor;
pub mod generation;

/// Имена инструментов, чьи lock-файлы раннер считает своими. Прежнее имя `v8-runner`
/// принимается: заброшенный lock прежнего бинарника форка — «свой», его разбирает
/// обычная логика живого владельца, а не ветка чужого формата.
pub fn is_known_tool_name(tool: &str) -> bool {
    tool == "v8-legacy-runner" || tool == "v8-runner"
}

#[cfg(test)]
mod tests {
    /// Lock прежнего бинарника форка остаётся своим после переименования.
    #[test]
    fn a_lock_of_the_previous_fork_binary_is_still_ours() {
        assert!(super::is_known_tool_name("v8-legacy-runner"));
        assert!(super::is_known_tool_name("v8-runner"));
        assert!(!super::is_known_tool_name("someone-else"));
    }
}
