//! Пол поддерживаемых платформ в форке: 8.3.12 (УТ10 работает на 8.3.12.1685).
//!
//! Апстрим держит минимум 8.3.24: у `push` ниже неё нет частичной загрузки, а незнакомая
//! команда платформы бывает не отказом, а зависанием. Форк принимает эти риски для
//! 8.3.12–8.3.23 как рабочие (зонд поколения закрыт `legacy::generation`); сам механизм
//! отказа апстрима — `refuse_below_minimum` — работает на нашем значении без изменений.

use crate::platform::locator::{platform_version_of, PlatformVersion, UtilityType};
use crate::platform::utilities::PlatformUtilities;

/// Платформа, с которой раннер пробует частичную загрузку наборов-расширений: 8.3.24.
///
/// Ниже неё у платформы нет самих ключей `-partial` (`/LoadConfigFromFiles`), `--partial`
/// (агент) и `--partial` (`ibcmd config import files`) — карта версий
/// (`references/1c/platform-versions.md`); форма команды раннера вне контракта платформы.
/// Частичная загрузка расширения замерена только на 8.3.27 (#379, #471), а замер форка
/// 12.11.2026 на 8.3.12.1685: отказ «Загрузка не должна менять принадлежность основного
/// объекта конфигурации» на любом списке — и на одном модуле, и на структурном изменении,
/// с `-updateConfigDumpInfo` и без. Между 8.3.24 и 8.3.26 остаётся поведение апстрима без
/// собственного замера форка
/// (`INV.PLATFORM.AN-EXTENSION-LOADS-WHOLE-BELOW-THE-PARTIAL-LOAD-FLOOR`).
pub const EXTENSION_PARTIAL_LOAD_FIRST_PLATFORM: PlatformVersion = PlatformVersion {
    major: 8,
    minor: 3,
    patch: 24,
    build: 0,
};

/// Причина, которой план называет полную загрузку расширения на платформе ниже пола.
pub const EXTENSION_FULL_LOAD_REASON: &str =
    "full load required for extension source-set below the partial-load floor";

/// Пробует ли раннер частичную загрузку расширений на этой установке платформы: версия
/// читается из пути найденного Конфигуратора. Версии нет — пробуют, как у апстрима
/// (fail-open): отказ частичной загрузки — обычный отказ команды, не зависание, и его
/// обходят полной загрузкой `push <Набор> --force`.
pub fn extensions_load_partially(utilities: &mut PlatformUtilities) -> bool {
    let version = utilities
        .locate(UtilityType::V8)
        .ok()
        .and_then(|location| platform_version_of(UtilityType::V8, &location.path));
    loads_extensions_partially(version)
}

/// Сам признак, без поиска утилит: `Some` — версия найденного Конфигуратора, `None` —
/// версия не названа. Один и для размера отправки, и для плана шага, и для сборки
/// `make`/`convert` — два ручных экземпляра условия разъехались бы при rebase.
pub(crate) fn loads_extensions_partially(version: Option<PlatformVersion>) -> bool {
    version.is_none_or(|version| version >= EXTENSION_PARTIAL_LOAD_FIRST_PLATFORM)
}

#[cfg(test)]
mod tests {
    use crate::platform::locator::{PlatformVersion, MINIMUM_PLATFORM};

    use super::{loads_extensions_partially, EXTENSION_PARTIAL_LOAD_FIRST_PLATFORM};

    /// Пол форка ровно 8.3.12: значение шва 5 не должно уплыть при rebase.
    #[test]
    fn the_floor_is_our_legacy_minimum() {
        assert_eq!(
            MINIMUM_PLATFORM,
            PlatformVersion {
                major: 8,
                minor: 3,
                patch: 12,
                build: 0,
            }
        );
    }

    const fn version(major: u32, minor: u32, patch: u32) -> PlatformVersion {
        PlatformVersion {
            major,
            minor,
            patch,
            build: 0,
        }
    }

    /// Частичная загрузка расширений пробуется с 8.3.24: платформу старше пола не
    /// спрашивают частичной загрузкой (замер 12.11.2026 на 8.3.12), новее — спрашивают,
    /// а версии в пути нет — спрашивают, как у апстрима (fail-open).
    #[test]
    fn extensions_load_partially_from_8_3_24() {
        assert!(!loads_extensions_partially(Some(version(8, 3, 12))));
        assert!(!loads_extensions_partially(Some(version(8, 3, 23))));
        assert!(loads_extensions_partially(Some(version(8, 3, 24))));
        assert!(loads_extensions_partially(Some(version(8, 3, 27))));
        assert!(loads_extensions_partially(Some(version(8, 5, 4))));
        assert!(
            loads_extensions_partially(None),
            "a versionless installation keeps the upstream behavior"
        );
        assert_eq!(
            EXTENSION_PARTIAL_LOAD_FIRST_PLATFORM,
            version(8, 3, 24),
            "the boundary is the oldest platform with the -partial keys"
        );
    }
}
