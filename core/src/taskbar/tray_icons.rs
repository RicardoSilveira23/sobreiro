//! Which tray icons are shown.

/// Which of the two tray icons are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconVisibility {
    pub claude: bool,
    pub copilot: bool,
}

/// Copilot's icon is shown when Copilot shows and its picture is drawn. Claude's is shown when
/// Claude shows or Copilot's is not shown. Never both hidden: the icons are the way to the menu.
pub fn visibility(show_claude: bool, copilot_shown: bool, copilot_drawn: bool) -> IconVisibility {
    let copilot = copilot_shown && copilot_drawn;
    IconVisibility {
        claude: show_claude || !copilot,
        copilot,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decides_each_combination() {
        // (show_claude, copilot_shown, copilot_drawn) -> (claude, copilot)
        let table = [
            (false, false, false, true, false),
            (false, false, true, true, false),
            (false, true, false, true, false),
            (false, true, true, false, true),
            (true, false, false, true, false),
            (true, false, true, true, false),
            (true, true, false, true, false),
            (true, true, true, true, true),
        ];
        for (show_claude, shown, drawn, claude, copilot) in table {
            assert_eq!(
                visibility(show_claude, shown, drawn),
                IconVisibility { claude, copilot },
                "{show_claude} {shown} {drawn}"
            );
        }
    }

    #[test]
    fn one_icon_is_always_shown() {
        for bits in 0..8u8 {
            let shown = visibility(bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
            assert!(shown.claude || shown.copilot, "{bits}");
        }
    }
}
