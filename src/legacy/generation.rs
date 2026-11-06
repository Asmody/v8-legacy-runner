//! Знание о зонде поколения на легаси-платформах.

use std::path::Path;
use std::time::Duration;

use crate::domain::capability::Provider;
use crate::platform::locator::PlatformVersion;

/// Платформа, первой знающая команду поколения: `/GetConfigGenerationID` пакетного
/// Конфигуратора и `config generation-id` режима агента реализованы в 8.3.20 (список
/// изменений платформы 8.3.20, V8Update.htm). Платформа старше команды ключ не узнаёт:
/// пакетной операции в запуске нет, Конфигуратор открывает окно в интерактивном режиме и
/// сам не кончается ([замер](../../references/1c/confirmed-runtime-measurements.md)
/// 03.11.2026 на 8.3.12).
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

/// Знает ли утилита с таким путём команду поколения
/// (`INV.PLATFORM.A-PLATFORM-WITHOUT-THE-GENERATION-COMMAND-IS-NOT-ASKED`). Версия читается
/// из пути установки — компонент вида `8.3.12.1855`, как его читает и поиск утилит; самая
/// глубокая из годных. Версии в пути нет — вопрос задают: от запуска без конца, каким
/// отвечает платформа без команды, чтение защищено собственным пределом, ответа нет.
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{knows_generation_id, probe_precheck};

    #[test]
    fn a_platform_from_8_3_20_knows_the_generation_command() {
        let binary = Path::new("C:/Program Files/1cv8/8.3.20.2274/bin/1cv8.exe");
        assert!(knows_generation_id(binary));
        assert_eq!(probe_precheck(binary), None);
    }

    /// Команда поколения реализована в 8.3.20: платформу старше не спрашивают, новее —
    /// спрашивают, а версии в пути нет — спрашивают, защищённые пределом чтения.
    #[test]
    fn a_platform_older_than_8_3_20_knows_no_generation_command() {
        let designer_of = |version: &str| {
            PathBuf::from("C:")
                .join("Program Files")
                .join("1cv8")
                .join(version)
                .join("bin")
                .join("1cv8.exe")
        };
        assert!(!knows_generation_id(&designer_of("8.3.12.1855")));
        assert!(!knows_generation_id(&designer_of("8.3.19.9999")));
        assert_eq!(
            probe_precheck(&designer_of("8.3.12.1855")),
            Some(None),
            "the probe is not asked: the answer is ready"
        );

        assert!(knows_generation_id(&designer_of("8.3.20.2639")));
        assert!(knows_generation_id(&designer_of("8.3.27.2074")));
        assert!(knows_generation_id(Path::new(
            "/opt/1cv8/x86_64/8.5.4.1878/1cv8"
        )));

        let ibcmd_of = |version: &str| {
            PathBuf::from("/opt/1cv8/x86_64")
                .join(version)
                .join("ibcmd")
        };
        assert!(!knows_generation_id(&ibcmd_of("8.3.13.1809")));
        assert!(knows_generation_id(&ibcmd_of("8.3.20.2639")));

        assert!(
            knows_generation_id(Path::new("1cv8.exe")),
            "a versionless path is asked anyway: the read cap answers for it"
        );
        let nested = PathBuf::from("8.3.12.1855")
            .join("portable")
            .join("8.3.27.2074")
            .join("1cv8.exe");
        assert!(
            knows_generation_id(&nested),
            "the deepest version-like component wins"
        );
    }

    /// Предупреждение о несверенном поколении громкое и называет суть.
    #[test]
    fn the_unchecked_generation_warning_names_the_unanswered_base() {
        assert!(super::UNCHECKED_GENERATION_NOTE.contains("not checked"));
    }

    /// Версии в пути нет — судить не по чему, зонд запускается (защищён `PROBE_CAP`).
    #[test]
    fn a_path_without_a_version_degrades_to_asking() {
        let binary = Path::new("C:/tools/v8-legacy-runner/1cv8.exe");
        assert!(knows_generation_id(binary));
        assert_eq!(probe_precheck(binary), None);
    }
}
