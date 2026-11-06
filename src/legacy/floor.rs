//! Пол поддерживаемых платформ в форке: 8.3.12 (УТ10 работает на 8.3.12.1685).
//!
//! Апстрим держит минимум 8.3.24: у `push` ниже неё нет частичной загрузки, а незнакомая
//! команда платформы бывает не отказом, а зависанием. Форк принимает эти риски для
//! 8.3.12–8.3.23 как рабочие (зонд поколения закрыт `legacy::generation`); сам механизм
//! отказа апстрима — `refuse_below_minimum` — работает на нашем значении без изменений.

#[cfg(test)]
mod tests {
    use crate::platform::locator::{PlatformVersion, MINIMUM_PLATFORM};

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
}
