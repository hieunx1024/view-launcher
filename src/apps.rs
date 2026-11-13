use crate::launcher::{ItemType, LauncherItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCategoryFilter {
    All,
    Calendar,
    Applications,
    Actions,
    Messages,
    Help,
    Mail,
    Settings,
    Contacts,
}

impl AppCategoryFilter {
    pub fn from_chip_name(name: &str) -> Self {
        match name.trim().to_lowercase().as_str() {
            "calendar" => Self::Calendar,
            "applications" | "apps" => Self::Applications,
            "actions" => Self::Actions,
            "messages" | "chat" => Self::Messages,
            "help" => Self::Help,
            "mail" | "email" => Self::Mail,
            "settings" | "preferences" => Self::Settings,
            "contacts" | "people" => Self::Contacts,
            _ => Self::All,
        }
    }

    pub fn matches(self, item: &LauncherItem) -> bool {
        match self {
            Self::All => true,
            Self::Applications => item.item_type == ItemType::App,
            Self::Actions => matches!(item.item_type, ItemType::System | ItemType::Shell | ItemType::Theme),
            _ => item.item_type == ItemType::App && self.matches_app(item),
        }
    }

    fn desktop_tags(self) -> &'static [&'static str] {
        match self {
            Self::Calendar => &["Calendar"],
            Self::Messages => &["InstantMessaging", "Chat", "IRCClient", "VideoConference"],
            Self::Help => &["Documentation"],
            Self::Mail => &["Email"],
            Self::Settings => &["Settings", "DesktopSettings", "HardwareSettings"],
            Self::Contacts => &["ContactManagement"],
            _ => &[],
        }
    }

    fn matches_app(self, item: &LauncherItem) -> bool {
        if item.desktop_categories.as_deref().is_some_and(|categories| {
            categories.split(';').any(|tag| self.desktop_tags().contains(&tag))
        }) {
            return true;
        }

        // Shortcuts and some desktop entries only provide a name.
        let name = item.name.to_lowercase();
        let words = |keywords: &[&str]| {
            name.split(|ch: char| !ch.is_alphanumeric())
                .any(|word| keywords.contains(&word))
        };
        match self {
            Self::Calendar => words(&["calendar", "kalender", "korganizer"]) || name == "lịch",
            Self::Messages => words(&[
                "messages", "chat", "slack", "discord", "telegram", "teams", "viber",
                "zalo", "whatsapp", "messenger", "zoom", "signal", "element",
            ]) || name == "tin nhắn",
            Self::Help => words(&["help", "yelp", "khelpcenter"]) || name == "hướng dẫn",
            Self::Mail => words(&["mail", "email", "thunderbird", "outlook", "geary", "kmail"]),
            Self::Settings => words(&["settings", "preferences", "systemsettings"])
                || name.contains("control center") || name == "cài đặt",
            Self::Contacts => words(&["contacts", "kaddressbook"])
                || name.contains("address book") || name == "danh bạ",
            _ => false,
        }
    }

    pub(crate) fn specific_label(item: &LauncherItem) -> Option<&'static str> {
        let categories = [
            (Self::Calendar, "Calendar"),
            (Self::Messages, "Messages"),
            (Self::Help, "Help"),
            (Self::Mail, "Mail"),
            (Self::Settings, "Settings"),
            (Self::Contacts, "Contacts"),
        ];
        // A single label represents the first matching category, while the
        // filters can expose the app in every category it belongs to.
        categories.into_iter()
            .find_map(|(category, label)| category.matches(item).then_some(label))
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_uses_desktop_metadata_and_matches_whole_words() {
        let mut app = LauncherItem::new(
            "Evolution".into(), "evolution".into(), ItemType::App, None, false, None,
        );
        app.desktop_categories = Some("Office;Email;Calendar;ContactManagement;".into());
        assert!(AppCategoryFilter::Mail.matches(&app));
        assert!(AppCategoryFilter::Calendar.matches(&app));
        assert!(AppCategoryFilter::Contacts.matches(&app));
        assert!(!AppCategoryFilter::Messages.matches(&app));

        app.name = "Mailman".into();
        app.desktop_categories = None;
        assert!(!AppCategoryFilter::Mail.matches(&app));

        app.name = "Help".into();
        app.item_type = ItemType::System;
        assert!(AppCategoryFilter::Actions.matches(&app));
        assert!(!AppCategoryFilter::Help.matches(&app));
    }

    #[test]
    fn test_category_filter_from_chip() {
        assert_eq!(AppCategoryFilter::from_chip_name("Calendar"), AppCategoryFilter::Calendar);
        assert_eq!(AppCategoryFilter::from_chip_name("Applications"), AppCategoryFilter::Applications);
        assert_eq!(AppCategoryFilter::from_chip_name("Actions"), AppCategoryFilter::Actions);
        assert_eq!(AppCategoryFilter::from_chip_name("Messages"), AppCategoryFilter::Messages);
        assert_eq!(AppCategoryFilter::from_chip_name("Help"), AppCategoryFilter::Help);
        assert_eq!(AppCategoryFilter::from_chip_name("Mail"), AppCategoryFilter::Mail);
        assert_eq!(AppCategoryFilter::from_chip_name("Settings"), AppCategoryFilter::Settings);
        assert_eq!(AppCategoryFilter::from_chip_name("Contacts"), AppCategoryFilter::Contacts);
        assert_eq!(AppCategoryFilter::from_chip_name("Unknown"), AppCategoryFilter::All);
    }
}
