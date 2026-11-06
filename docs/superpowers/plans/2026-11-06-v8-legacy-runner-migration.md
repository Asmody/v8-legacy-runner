# Миграция в v8-legacy-runner — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Перевести форк на архитектуру дельты: ветка `legacy/8.3.12` поверх зеркала `master`, знание в `src/legacy/`, швы в файлах апстрима, идентичность `v8-legacy-runner`, регламент обновления.

**Architecture:** `master` — зеркало апстрима (только ff-only). Вся работа — в `legacy/8.3.12`: сначала в неё переносится существующая дельта (3 коммита фикса + спека), затем она переукладывается в форму «знания в `src/legacy/`, швы 1–8 строк в файлах апстрима». Затем — пол 8.3.12, громкое предупреждение о несверенном поколении, идентичность, CI, релиз и деплой.

**Tech Stack:** Rust (cargo, edition 2021, MSRV 1.95), GitHub API через `curl --ssl-no-revoke` + токен из `git credential fill`.

**Spec:** [docs/superpowers/specs/2026-11-06-v8-legacy-runner-overlay-design.md](../specs/2026-11-06-v8-legacy-runner-overlay-design.md)

## Global Constraints

- Пол платформ форка: **8.3.12** (`MINIMUM_PLATFORM { major: 8, minor: 3, patch: 12, build: 0 }`); апстримовский 8.3.24 не используется.
- Первая платформа с `/GetConfigGenerationID`: **8.3.20** (`GENERATION_ID_FIRST_PLATFORM`); платформу младше зонд не спрашивают.
- Предел зонда поколения: **30 с** (`PROBE_CAP`); снятый по пределу запуск — «нет ответа», не отказ.
- Версия форка: `<версия апстрима>-legacy.<N>` → сейчас **`0.16.0-legacy.1`**.
- CLI, флаги, формат `v8project.yaml` — без изменений.
- В `master` не коммитить никогда; всё — в `legacy/8.3.12`.
- Новое знание дельты — только в `src/legacy/`; в файлах апстрима — только швы (вызовы `crate::legacy::*`, значения констант, тесты).
- Логирование: подписчик по умолчанию `info` — `tracing::warn!` виден пользователю в консоли.
- `cargo test` на этой машине даёт ~83 средозависимых падения независимо от изменений; сверять набором, а не нулём. `cargo test --lib` не работает (bin-only крейт) — использовать `cargo test --bin v8-runner` (до переименования бинарника; после — `--bin v8-legacy-runner`).
- Документация и комментарии — на английском в коде (стиль апстрима), спеки/планы — на русском.

## Review Focus

1. **Медленный ответ зонда на платформе ≥ 8.3.20** — зонд снимается капой 30 с, это «нет ответа» (`warn!`), а не отказ; обмен не должен прерваться. Тест: `a_generation_read_past_its_cap_is_no_answer` (designer/ibcmd, Task 2).
2. **Версия платформы не читается из пути** — `knows_generation_id` деградирует к «спрашиваем», защищает капа. Тест: `a_platform_older_than_8_3_20_knows_no_generation_command` + тест деградации в Task 2.
3. **Граница пола ровно 8.3.12** — 8.3.12.0 принимается, 8.3.11.9999 отказывается. Тесты: обновлённые тесты минимума (Task 3) + `the_floor_is_our_legacy_minimum`.
4. **`push --force` не должен получать ложное предупреждение о несверенности** — в `compare()` ветка Force возвращается до чтения токена (структура апстрима), шов стоит только в ветке «ответа нет» при наличии записи. Тест `the_unchecked_generation_warning_names_the_unanswered_base` (Task 4) фиксирует текст; не-срабатывание на Force гарантировано местом шва — при rebase шов не переносить из этой ветки.
5. **Запись поколения после загрузки при отсутствии ответа** — поведение апстрима (стереть/не записать) не меняется; шов только добавляет `warn!`. Покрыто существующими тестами апстрима `a_failed_load_marks_the_generation_record` / `a_load_records_its_generation_or_erases_the_record`.

---

### Task 1: Ветка `legacy/8.3.12` — перенос дельты

**Files:**
- Create: ветка `legacy/8.3.12` от `master` (9b6c5d8), cherry-pick 4 коммитов

**Interfaces:**
- Consumes: коммиты `680b6f8`, `43fe531`, `a94a86a`, `f9bc0a8` в `fix/generation-probe-old-platforms`
- Produces: ветка `legacy/8.3.12` с дельтой в inline-форме, зелёная сборка

- [ ] **Step 1: Создать ветку и перенести коммиты**

```bash
cd /c/w/v8-runner-rust
git checkout master
git checkout -b legacy/8.3.12
git cherry-pick 680b6f8 43fe531 a94a86a f9bc0a8
```

Ожидание: 4 коммита применяются без конфликтов (merge-tree probe уже дал 0 конфликтов).

- [ ] **Step 2: Проверить сборку и целевые тесты**

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test --bin v8-runner generation
```

Ожидание: fmt/clippy чисто; тесты `generation` проходят. Полный `cargo test --bin v8-runner` — прогнать и сравнить число падений с известными ~83 средозависимыми (должно совпасть).

- [ ] **Step 3: Commit (если были конфликты — их разрешение)**

Cherry-pick уже создаёт коммиты. Если конфликтов не было — шаг пропускается.

---

### Task 2: `src/legacy/generation.rs` + `mod.rs` — знания о зонде, швы 1–4

**Files:**
- Create: `src/legacy/mod.rs`, `src/legacy/generation.rs`
- Modify: `src/platform/generation.rs` (вернуть апстрим-вид), `src/platform/designer.rs` (`config_generation_id`), `src/platform/ibcmd.rs` (`config_generation_id`), `src/use_cases/generation_reader.rs` (2 места), `spec/rules/platform/a-platform-without-the-generation-command-is-not-asked.md`, `spec/rules/use-cases/a-generation-read-declares-its-own-cap.md`

**Interfaces:**
- Consumes: тесты дельты из Task 1
- Produces: `crate::legacy::generation::{GENERATION_ID_FIRST_PLATFORM, knows_generation_id(binary: &Path) -> bool, probe_precheck(binary: &Path) -> Option<Option<String>>, PROBE_CAP: Duration}`

- [ ] **Step 1: Создать `src/legacy/mod.rs` с таблицей швов**

```rust
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
//! | 1 | `platform/generation.rs` | без изменений (апстрим-вид) |
//! | 2 | `platform/designer.rs::config_generation_id` | precheck `probe_precheck` + плечо `TimedOut` → `warn!` + `Ok(None)` |
//! | 3 | `platform/ibcmd.rs::config_generation_id` | то же, что #2 |
//! | 4 | `use_cases/generation_reader.rs` (оба места `process_policy`) | `Some(crate::legacy::generation::PROBE_CAP)` |
//! | 5 | `platform/locator.rs::MINIMUM_PLATFORM` | 8.3.24 → 8.3.12, комментарий ведёт в `legacy::floor` |
//! | 6 | `use_cases/exchange_guard.rs::compare` | ветка «ответа нет» → `warn_unchecked` |
//! | 7 | тесты минимума: `locator.rs`, `use_cases/result.rs`, `tests/cli_minimum_platform.rs` | фиксуры 8.3.23 → 8.3.11, 8.3.24 → 8.3.12 |

pub mod floor;
pub mod generation;
```

(На этом шаге `pub mod floor;` закомментировать — `floor.rs` появится в Task 3.)

- [ ] **Step 2: Создать `src/legacy/generation.rs` с тестом деградации**

Перенести сюда `GENERATION_ID_FIRST_PLATFORM`, `knows_generation_id`, тест `a_platform_older_than_8_3_20_knows_no_generation_command` из `src/platform/generation.rs` и `PROBE_CAP` из `src/use_cases/generation_reader.rs`. Добавить `probe_precheck` и тест деградации (Review Focus 2). Путь импорта `PlatformVersion` взять фактический из переехавшего кода (в апстриме тип определён в `locator.rs`):

```rust
//! Знание о зонде поколения на легаси-платформах.

use std::path::Path;
use std::time::Duration;

use crate::platform::locator::PlatformVersion;

/// Первая платформа, чей пакетный Конфигуратор знает `/GetConfigGenerationID`.
/// На 8.3.12 ключ не узнан: Конфигуратор уходит в интерактив и висит
/// ([замер](../../references/1c/confirmed-runtime-measurements.md), 03.11.2026).
const GENERATION_ID_FIRST_PLATFORM: PlatformVersion = PlatformVersion {
    major: 8,
    minor: 3,
    patch: 20,
    build: 0,
};

/// Собственный предел чтения поколения
/// (`INV.USE-CASES.A-GENERATION-READ-DECLARES-ITS-OWN-CAP`): снятый запуск —
/// отсутствие ответа, не отказ.
pub const PROBE_CAP: Duration = Duration::from_secs(30);

/// Знает ли этот бинарник команду `/GetConfigGenerationID`. Версия читается из пути
/// утилиты; если там версии нет — считаем, что знает: зонд защищён `PROBE_CAP`.
pub fn knows_generation_id(binary: &Path) -> bool {
    binary
        .components()
        .rev()
        .filter_map(|component| component.as_os_str().to_str())
        .find_map(PlatformVersion::parse_strict)
        .is_none_or(|version| version >= GENERATION_ID_FIRST_PLATFORM)
}

/// Пречек зонда: платформа младше 8.3.20 команды не знает, спрашивать нельзя —
/// Конфигуратор уходит в интерактив и его запуск конца не имеет.
/// `Some(ответ)` — зонд не запускается, ответ готов; `None` — зонд запускается,
/// как у апстрима.
pub fn probe_precheck(binary: &Path) -> Option<Option<String>> {
    if knows_generation_id(binary) {
        return None;
    }
    tracing::debug!(
        binary = %binary.display(),
        "the probe is not asked: the platform knows no generation command"
    );
    Some(None)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{knows_generation_id, probe_precheck};

    #[test]
    fn a_platform_older_than_8_3_20_knows_no_generation_command() {
        let binary = Path::new("C:/Program Files/1cv8/8.3.12.1685/bin/1cv8.exe");
        assert!(!knows_generation_id(binary));
        assert_eq!(probe_precheck(binary), Some(None));
    }

    #[test]
    fn a_platform_from_8_3_20_knows_the_generation_command() {
        let binary = Path::new("C:/Program Files/1cv8/8.3.20.2274/bin/1cv8.exe");
        assert!(knows_generation_id(binary));
        assert_eq!(probe_precheck(binary), None);
    }

    /// Версии в пути нет — судить не по чему, зонд запускается (защищён `PROBE_CAP`).
    #[test]
    fn a_path_without_a_version_degrades_to_asking() {
        let binary = Path::new("C:/tools/v8-legacy-runner/1cv8.exe");
        assert!(knows_generation_id(binary));
        assert_eq!(probe_precheck(binary), None);
    }
}
```

- [ ] **Step 3: Вернуть `src/platform/generation.rs` к апстрим-виду (шов 1)**

```bash
git checkout master -- src/platform/generation.rs
```

- [ ] **Step 4: Шов 2 — `designer.rs::config_generation_id`**

Заменить inline-гейт и `self.run(&args)?` на:

```rust
    pub fn config_generation_id(
        &self,
        extension: Option<&str>,
    ) -> Result<Option<String>, DesignerError> {
        if self.log_file.is_none() {
            return Ok(None);
        }
        if let Some(answer) = crate::legacy::generation::probe_precheck(&self.binary) {
            return Ok(answer);
        }
        let mut args = self.base_args();
        args.push("/GetConfigGenerationID".to_owned());
        if let Some(extension) = extension {
            args.push("-Extension".to_owned());
            args.push(extension.to_owned());
        }
        let result = match self.run(&args) {
            Ok(result) => result,
            Err(DesignerError::Spawn(ProcessError::TimedOut { timeout_ms, .. })) => {
                tracing::warn!(
                    binary = %self.binary.display(),
                    timeout_ms,
                    "the generation probe hit its cap: no answer"
                );
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if result.process.exit_code != 0 {
            return Ok(None);
        }
        Ok(result
            .platform_log
            .as_deref()
            .and_then(crate::platform::generation::generation_token))
    }
```

Тесты `generation_tests` (RefusingRunner/TimedOutRunner/FailingRunner) остаются в файле — это шовные тесты.

- [ ] **Step 5: Шов 3 — `ibcmd.rs::config_generation_id` (то же)**

```rust
    pub fn config_generation_id(
        &self,
        extension: Option<&str>,
    ) -> Result<Option<String>, IbcmdError> {
        if let Some(answer) = crate::legacy::generation::probe_precheck(&self.binary) {
            return Ok(answer);
        }
        let mut args = self.authenticated_infobase_args(&["config", "generation-id"]);
        if let Some(extension) = extension {
            push_option_value(&mut args, "--extension", extension);
        }
        let result = match self.run_with(&args, &self.execution_policy.for_reading()) {
            Ok(result) => result,
            Err(IbcmdError::Spawn(ProcessError::TimedOut { timeout_ms, .. })) => {
                tracing::warn!(
                    binary = %self.binary.display(),
                    timeout_ms,
                    "the generation probe hit its cap: no answer"
                );
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if result.process.exit_code != 0 {
            return Ok(None);
        }
        Ok(crate::platform::generation::generation_token(
            &result.process.stdout,
        ))
    }
```

- [ ] **Step 6: Шов 4 — оба места `process_policy` в `generation_reader.rs`**

Оба вызова (строки 59 и 203 апстрима):

```rust
let policy = context.process_policy(
    InterruptionSafetyClass::GracefulThenKill,
    Some(crate::legacy::generation::PROBE_CAP),
);
```

Локальную `const GENERATION_PROBE_CAP` удалить; в тестах `generation_reader.rs` ссылки на неё заменить на `crate::legacy::generation::PROBE_CAP`.

- [ ] **Step 7: Обновить чеки правил**

`spec/rules/platform/a-platform-without-the-generation-command-is-not-asked.md`: чек теста знания → `src/legacy/generation.rs::a_platform_older_than_8_3_20_knows_no_generation_command`; в тексте путь знания — `src/legacy/generation.rs`.

`spec/rules/use-cases/a-generation-read-declares-its-own-cap.md`: упоминание `generation_reader::GENERATION_PROBE_CAP` → `legacy::generation::PROBE_CAP`; чеки тестов `generation_reader.rs` и designer/ibcmd остаются.

- [ ] **Step 8: Проверить**

```bash
cargo fmt
cargo clippy --all-targets
cargo test --bin v8-runner generation
git diff master --stat -- src/platform/generation.rs
```

Ожидание: тесты проходят; `git diff master -- src/platform/generation.rs` **пуст** (шов 1 = апстрим-вид).

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor(legacy): знания о зонде поколения переезжают в src/legacy, швы 1-4"
```

---

### Task 3: `src/legacy/floor.rs` + шов 5 — пол 8.3.12

**Files:**
- Create: `src/legacy/floor.rs`
- Modify: `src/platform/locator.rs` (константа `MINIMUM_PLATFORM`, doc-комментарий, 3 теста), `src/use_cases/result.rs` (1 тест), `tests/cli_minimum_platform.rs`, `spec/rules/platform/a-platform-below-the-minimum-is-refused.md`, раскомментировать `pub mod floor;` в `src/legacy/mod.rs`

**Interfaces:**
- Consumes: `crate::platform::locator::{MINIMUM_PLATFORM, PlatformVersion}`
- Produces: `crate::legacy::floor` с тестом значения пола; `MINIMUM_PLATFORM == 8.3.12.0`

- [ ] **Step 1: Шов 5 — значение и комментарий**

`src/platform/locator.rs:79`:

```rust
/// Первая поддерживаемая платформа: у апстрима 8.3.24, у форка — 8.3.12
/// (`crate::legacy::floor`, спека `docs/superpowers/specs/`).
pub const MINIMUM_PLATFORM: PlatformVersion = PlatformVersion {
    major: 8,
    minor: 3,
    patch: 12,
    build: 0,
};
```

- [ ] **Step 2: Создать `src/legacy/floor.rs`**

```rust
//! Пол поддерживаемых платформ в форке: 8.3.12 (УТ10 работает на 8.3.12.1685).
//!
//! Апстрим держит минимум 8.3.24: у `push` ниже неё нет частичной загрузки, а незнакомая
//! команда платформы бывает не отказом, а зависанием. Форк принимает эти риски для
//! 8.3.12–8.3.23 как рабочие (зонд поколения закрыт `legacy::generation`); сам механизм
//! отказа апстрима — `refuse_below_minimum` — работает на нашем значении без изменений.

use crate::platform::locator::{MINIMUM_PLATFORM, PlatformVersion};

#[cfg(test)]
mod tests {
    use crate::platform::locator::MINIMUM_PLATFORM;

    /// Пол форка ровно 8.3.12: значение шва 5 не должно уплыть при rebase.
    #[test]
    fn the_floor_is_our_legacy_minimum() {
        assert_eq!(
            MINIMUM_PLATFORM,
            crate::platform::locator::PlatformVersion {
                major: 8,
                minor: 3,
                patch: 12,
                build: 0,
            }
        );
    }
}
```

Раскомментировать `pub mod floor;` в `src/legacy/mod.rs`.

- [ ] **Step 3: Обновить тесты минимума (шов 7)**

Правило замены в тестах: «ниже минимума» = `8.3.23…` → `8.3.11…`; «минимум» = `8.3.24` → `8.3.12`:

- `src/platform/locator.rs` тест `a_platform_below_the_minimum_is_refused_and_the_minimum_is_accepted`: `"8.3.23.9999"` → `"8.3.11.9999"`, `"8.3.24.1"` → `"8.3.12.1"`, `locator_for("8.3.23")` → `locator_for("8.3.11")`, `locator_for("8.3.24")` → `locator_for("8.3.12")`, ожидание сообщения `"platform 8.3.24 and newer"` → `"platform 8.3.12 and newer"`; doc-комментарий теста аналогично.
- `src/platform/locator.rs` тест `an_explicit_path_does_not_bypass_the_minimum`: каталог `"8.3.12.1855"` → `"8.3.11.1855"` (8.3.12 теперь проходит минимум).
- `src/use_cases/result.rs` тест с `BelowMinimumPlatform` (~строки 742–766): `8.3.24.1000` → `8.3.12.1000` в конструировании, `8.3.23.1000` → `8.3.11.1000`, `contains("8.3.24")` → `contains("8.3.12")`.
- `tests/cli_minimum_platform.rs`: все `"8.3.23.9999"` → `"8.3.11.9999"`, `"8.3.24.1819"` → `"8.3.12.1819"`, `"platform 8.3.24 and newer"` → `"platform 8.3.12 and newer"`.

Другие вхождения `8.3.24.*` в тестах locator.rs (фикстуры 1716, 2119, 2216, 2795, 2823, 2893, 2914) не трогать — они не о минимуме и проходят с любым полом ≤ 8.3.24.

- [ ] **Step 4: Обновить правило `a-platform-below-the-minimum-is-refused.md`**

Первую строку «Минимальная поддерживаемая версия платформы — **8.3.24** (решение владельца)» заменить:

```markdown
Минимальная поддерживаемая версия платформы — **8.3.12** (решение владельца форка
`v8-legacy-runner`; апстрим держит 8.3.24, знание пола — в `crate::legacy::floor`).
```

- [ ] **Step 5: Проверить**

```bash
cargo fmt
cargo clippy --all-targets
cargo test --bin v8-runner the_floor_is_our_legacy_minimum minimum
cargo test --test cli_minimum_platform
```

Ожидание: все зелёные.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(legacy): пол платформ 8.3.12 (шов MINIMUM_PLATFORM) + floor.rs"
```

---

### Task 4: Шов 6 — несверенное поколение видно (`warn_unchecked`)

**Files:**
- Modify: `src/legacy/generation.rs` (добавить `warn_unchecked`), `src/use_cases/exchange_guard.rs` (`compare`, ветка «ответа нет»)

**Interfaces:**
- Consumes: `exchange_guard::GenerationGate::compare` — ветка `let Some(token) = read()? else { ... }`
- Produces: `crate::legacy::generation::{UNCHECKED_GENERATION_NOTE: &str, warn_unchecked(set: &str)}`

- [ ] **Step 1: Тест**

В `src/legacy/generation.rs` (модуль tests):

```rust
    /// Предупреждение о несверенном поколении громкое и называет суть.
    #[test]
    fn the_unchecked_generation_warning_names_the_unanswered_base() {
        assert!(super::UNCHECKED_GENERATION_NOTE.contains("not checked"));
    }
```

- [ ] **Step 2: Запустить, убедиться в падении**

```bash
cargo test --bin v8-runner the_unchecked_generation_warning
```

Ожидание: FAIL — `UNCHECKED_GENERATION_NOTE` не определена.

- [ ] **Step 3: Реализация в `src/legacy/generation.rs`**

```rust
use crate::domain::capability::Provider;

/// Пользовательский текст несверенности: обмен идёт, но сверить поколение не с чем.
pub const UNCHECKED_GENERATION_NOTE: &str =
    "the configuration generation is not checked: the tool gave no answer";

/// Несверенное поколение видно
/// (`INV.USE-CASES.WITHOUT-A-GENERATION-ANSWER-A-NO-MEMORY-REFUSAL-OFFERS-PULL-FORCE`,
/// замечания ревью PR #503): база не ответила поколением — обмен идёт, но в журнал идёт
/// громкое предупреждение. Ветка Force сюда не доходит: она возвращается до чтения токена.
pub fn warn_unchecked(set: &str, tool: Provider) {
    tracing::warn!(set, tool = ?tool, "{}", UNCHECKED_GENERATION_NOTE);
}
```

- [ ] **Step 4: Шов 6 в `exchange_guard.rs::compare`**

```rust
        let Some(token) = read()? else {
            crate::legacy::generation::warn_unchecked(set.name(), tool);
            return Ok(BeforeLoad::Unchecked);
        };
```

- [ ] **Step 5: Проверить**

```bash
cargo fmt
cargo clippy --all-targets
cargo test --bin v8-runner exchange_guard generation
```

Ожидание: зелёные; существующее поведение `Unchecked` не меняется (тесты апстрима проходят).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(legacy): громкое предупреждение о несверенном поколении при отсутствии ответа"
```

---

### Task 5: Идентичность `v8-legacy-runner`

**Files:**
- Modify: `Cargo.toml`, `FORK_NOTICE.md`, `README.md`, `.github/workflows/release.yml:24`
- (проверить) `src/legacy/mod.rs` — упоминания имени пакета

**Interfaces:**
- Produces: пакет и бинарник `v8-legacy-runner`, версия `0.16.0-legacy.1`

- [ ] **Step 1: `Cargo.toml`**

```toml
name = "v8-legacy-runner"
version = "0.16.0-legacy.1"
...
repository = "https://github.com/Asmody/v8-legacy-runner"

[[bin]]
name = "v8-legacy-runner"
path = "src/main.rs"
```

Остальное (`edition`, `rust-version`, лицензия, зависимости) — не трогать.

- [ ] **Step 2: `FORK_NOTICE.md` — переписать**

```markdown
# v8-legacy-runner fork notice

This repository, `v8-legacy-runner`, is the maintained fork of
[`IngvarConsulting/v8-runner-rust`](https://github.com/IngvarConsulting/v8-runner-rust)
that keeps 1C:Enterprise 8.3.12–8.3.23 working.

- Fork established: 2026-11-06.
- Upstream baseline: `9b6c5d8` (upstream `master`, minimum platform 8.3.24).
- Maintained source: <https://github.com/Asmody/v8-legacy-runner>.
- License: GNU Affero General Public License v3.0 only (`AGPL-3.0-only`).

The fork carries a delta over the upstream, laid out as knowledge in `src/legacy/`
plus thin seams in upstream files (the table of seams lives in `src/legacy/mod.rs`).
`master` mirrors the upstream and is only fast-forwarded; all work happens on
`legacy/8.3.12`. The update procedure from the upstream is described in `AGENTS.md`,
section "Legacy delta over the upstream".
```

- [ ] **Step 3: `README.md` — преамбула**

Добавить сразу после заголовка:

```markdown
> **`v8-legacy-runner`** — форк [`v8-runner`](https://github.com/IngvarConsulting/v8-runner-rust),
> поддерживающий платформы 1С 8.3.12–8.3.23 (апстрим держит минимум 8.3.24).
> CLI и конфигурация совпадают с апстримом; дельта описана в
> [`FORK_NOTICE.md`](FORK_NOTICE.md) и [`src/legacy/mod.rs`](src/legacy/mod.rs).
```

- [ ] **Step 4: `release.yml` — имя бинарника**

Строку 24 `BIN_NAME: v8-runner` → `BIN_NAME: v8-legacy-runner`. Остальные упоминания `v8-runner` в текстах body (описания таблиц) — не трогать (косметика, не влияет на сборку).

- [ ] **Step 5: Проверить сборку с новым именем**

```bash
git grep -rn "CARGO_BIN_EXE" tests/ src/ | head
cargo fmt
cargo clippy --all-targets
cargo build
```

Ожидание: если в тестах есть `env!("CARGO_BIN_EXE_v8-runner")` — обновить на `v8-legacy-runner` (макрос не параметризуем, требует точного имени). Сборка зелёная.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore(legacy): идентичность v8-legacy-runner (пакет, бинарник, версия 0.16.0-legacy.1)"
```

---

### Task 6: GitHub — переименование репо, push, закрытие PR #1

**Files:** только GitHub-side операции.

**Interfaces:**
- Consumes: токен GitHub из `git credential fill` (не печатать, не сохранять)
- Produces: `Asmody/v8-legacy-runner`, запушенная `legacy/8.3.12`, закрытый PR #1

- [ ] **Step 1: Переименовать репо через API**

```bash
TOKEN=$(printf 'protocol=https\nhost=github.com\n' | git credential fill | grep '^password=' | cut -d= -f2-)
curl -sS --ssl-no-revoke -X PATCH \
  -H "Authorization: Bearer $TOKEN" -H "Accept: application/vnd.github+json" \
  https://api.github.com/repos/Asmody/v8-runner-rust \
  -d '{"name":"v8-legacy-runner"}' | grep -m1 '"full_name"'
```

Ожидание: `"full_name": "Asmody/v8-legacy-runner"`.

- [ ] **Step 2: Обновить remote и запушить**

```bash
git remote set-url origin https://github.com/Asmody/v8-legacy-runner.git
git push origin legacy/8.3.12
```

- [ ] **Step 3: Закрыть PR #1 с комментарием**

```bash
curl -sS --ssl-no-revoke -X PATCH \
  -H "Authorization: Bearer $TOKEN" -H "Accept: application/vnd.github+json" \
  https://api.github.com/repos/Asmody/v8-runner-rust/issues/1 \
  -d '{"state":"closed"}' > /dev/null
printf '%s' '{"body":"Содержимое переложено в ветку `legacy/8.3.12` в форме швов над апстримом (см. спеку `docs/superpowers/specs/2026-11-06-v8-legacy-runner-overlay-design.md`). Ветка удалена."}' | \
curl -sS --ssl-no-revoke -X POST \
  -H "Authorization: Bearer $TOKEN" -H "Accept: application/vnd.github+json" \
  https://api.github.com/repos/Asmody/v8-runner-rust/issues/1/comments -d @- > /dev/null
```

- [ ] **Step 4: Удалить старую ветку в origin**

```bash
git push origin --delete fix/generation-probe-old-platforms
git branch -D fix/generation-probe-old-platforms
```

---

### Task 7: CI-проверка апстрима + правила проекта

**Files:**
- Create: `.github/workflows/legacy-upstream-check.yml`
- Modify: `AGENTS.md` (раздел «Legacy delta over the upstream»)

**Interfaces:**
- Produces: еженедельный merge-tree probe; регламент в правилах проекта

- [ ] **Step 1: `.github/workflows/legacy-upstream-check.yml`**

```yaml
name: legacy-upstream-check

on:
  schedule:
    - cron: "0 4 * * 1" # еженедельно, понедельник 04:00 UTC
  workflow_dispatch:

permissions:
  issues: write

jobs:
  probe:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Fetch upstream
        run: |
          git remote add upstream https://github.com/IngvarConsulting/v8-runner-rust.git
          git fetch upstream master
      - name: Fast-forward mirror check
        run: |
          if ! git merge-base --is-ancestor master upstream/master; then
            echo "master diverged from upstream/master" | tee probe.txt
          fi
      - name: Trial merge of the delta
        run: |
          if git merge-tree --write-tree master legacy/8.3.12 > tree.txt 2>&1; then
            echo "clean"
          else
            echo "the delta conflicts with the upstream" | tee -a probe.txt
            cat tree.txt >> probe.txt
          fi
      - name: New upstream movement
        run: |
          count=$(git rev-list --count master..upstream/master)
          echo "new upstream commits: $count" >> probe.txt
      - name: Open an issue on conflict or movement
        uses: actions/github-script@v7
        with:
          script: |
            const fs = require('fs');
            const probe = fs.readFileSync('probe.txt', 'utf8');
            const needs = probe.includes('diverged') || probe.includes('conflicts');
            const movement = /new upstream commits: (\d+)/.exec(probe);
            if (!needs && (!movement || movement[1] === '0')) return;
            const title = needs ? 'Дельта конфликтует с апстримом' : 'Апстрим двинулся: обновите legacy/8.3.12';
            await github.rest.issues.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              title,
              body: '```\n' + probe + '\n```\nПроцедура: `AGENTS.md`, раздел "Legacy delta over the upstream".'
            });
```

Никаких auto-push/auto-rebase/auto-merge — только issue.

- [ ] **Step 2: `AGENTS.md` — раздел в конце файла**

```markdown
## Legacy delta over the upstream

This repository is the fork `v8-legacy-runner` of
[`IngvarConsulting/v8-runner-rust`](https://github.com/IngvarConsulting/v8-runner-rust)
keeping platform 8.3.12–8.3.23 alive. The rules:

- `master` mirrors the upstream, fast-forward only, never commit to it;
- all work, specs and releases happen on `legacy/8.3.12`;
- delta knowledge lives in `src/legacy/` — the table of seams in
  [`src/legacy/mod.rs`](src/legacy/mod.rs) is the canonical list of what to re-apply
  when updating from the upstream; upstream files carry only 1–8 line seams;
- the design: `docs/superpowers/specs/2026-11-06-v8-legacy-runner-overlay-design.md`.

Updating from the upstream (run it **before** changing code):

1. `git fetch upstream`
2. `git checkout master && git merge --ff-only upstream/master && git push origin master`
3. `git checkout legacy/8.3.12 && git rebase master`
4. Conflicts are resolved in the seams only (see the table in `src/legacy/mod.rs`):
   fix the seam, never inline logic into an upstream file. If the upstream changed the
   very place a seam touches (took the cap, reworked the probe) — the upstream version
   wins: resolve in its favor, re-seat or drop the seam.
5. `cargo fmt && cargo clippy --all-targets && cargo test`
6. `Cargo.toml`: `version = <upstream version>-legacy.<N+1>` (the `N` resets to 1 on
   each upstream update)
7. `cargo build --release`, tag `legacy/v<version>`, deploy the binary, update the
   consuming project's runbook if the binary name or CLI changed
8. `git push --force-with-lease origin legacy/8.3.12`

A weekly CI probe (`legacy-upstream-check`) opens an issue when the upstream moved or
the delta conflicts; it never pushes or rebases anything itself.
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "docs(legacy): регламент обновления из апстрима + еженедельный CI-probe"
```

---

### Task 8: Релиз, деплой, живая проверка

**Files:**
- Create: тег `legacy/v0.16.0-legacy.1`
- Deploy: `C:\tools\v8-legacy-runner\v8-legacy-runner.exe`
- Modify (другой репо): `C:\w\УТ10_master\docs\memory\proc.md`

**Interfaces:**
- Consumes: собранный бинарник; УТ10 `v8project.local.yaml` (`tools.platform.path` указывает на 8.3.12.1685)

- [ ] **Step 1: Собрать релиз**

```bash
cargo build --release
```

Ожидание: `target/release/v8-legacy-runner.exe`; `v8-legacy-runner.exe --version` → `v8-legacy-runner 0.16.0-legacy.1`.

- [ ] **Step 2: Тег и push**

```bash
git tag legacy/v0.16.0-legacy.1
git push origin legacy/v0.16.0-legacy.1
```

- [ ] **Step 3: Деплой**

```bash
mkdir -p /c/tools/v8-legacy-runner
cp target/release/v8-legacy-runner.exe /c/tools/v8-legacy-runner/
```

- [ ] **Step 4: Обновить `C:\w\УТ10_master\docs\memory\proc.md`**

Первый пункт (и его подпункт про зависание) заменить на:

```markdown
- Запуск YAxUnit-тестов - напрямую `C:\tools\v8-legacy-runner\v8-legacy-runner.exe` (проект уже настроен: `v8project.yaml` + `v8project.local.yaml` в корне репозитория).
  - Форк `Asmody/v8-legacy-runner` (версия 0.16.0-legacy.1, ветка `legacy/8.3.12`, собран 06.11.2026): дельта поверх апстрима IngvarConsulting/v8-runner-rust ради платформы 8.3.12. Раннер не спрашивает поколение конфигурации у платформы старше... младше 8.3.20 (ключ `/GetConfigGenerationID` не реализован - Конфигуратор уходил в интерактив и висел), чтение ограничено собственным пределом 30 с; при отсутствии ответа в журнале ожидаемо `WARN` "the configuration generation is not checked" - обмен идёт. Старое имя `v8-runner.exe` и обёртка `v8-runner.cmd` + `designer-guard.ps1` устарели (файлы пока в `C:\tools\v8-runner`, старый exe - рядом как `v8-runner.exe.bak`).
```

- [ ] **Step 5: Живая проверка на базах УТ10**

```bash
cd /c/w/УТ10_master
/c/tools/v8-legacy-runner/v8-legacy-runner.exe status --deep origin
/c/tools/v8-legacy-runner/v8-legacy-runner.exe status --deep test
/c/tools/v8-legacy-runner/v8-legacy-runner.exe --infobase test push Тесты --force
```

Ожидание: без зависаний (status 5–7 с, push ~13 с); в логе push «the probe is not asked»; предупреждающих висящих процессов 1cv8 нет.

- [ ] **Step 6: Финальный push ветки**

```bash
cd /c/w/v8-runner-rust && git push --force-with-lease origin legacy/8.3.12
```

- [ ] **Step 7: Обновить личную память (`v8-runner-fork-and-8312-fix.md`)** — миграция выполнена, пути/имена новые, PR #1 закрыт.
