/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use superconsole::style::Color;
use superconsole::style::ContentStyle;
use superconsole::style::StyledContent;

/// A way to uniformly print to the console after a command has finished. This should
/// only be used at the end of a command, after the event context from the buckd client
/// is not available.
pub struct FinalConsole {
    is_tty: bool,
}

impl FinalConsole {
    pub fn new_with_tty() -> Self {
        Self { is_tty: true }
    }

    pub fn new_without_tty() -> Self {
        Self { is_tty: false }
    }

    fn stderr_colored_ln(&self, message: &str, color: Color) -> buck2_error::Result<()> {
        self.stderr_colored(message, color)?;
        crate::eprintln!()?;
        Ok(())
    }

    fn stderr_colored(&self, message: &str, color: Color) -> buck2_error::Result<()> {
        if self.is_tty {
            let sc = StyledContent::new(
                ContentStyle {
                    foreground_color: Some(color),
                    background_color: None,
                    underline_color: None,
                    attributes: Default::default(),
                },
                message,
            );
            crate::eprint!("{}", sc)?;
        } else {
            crate::eprint!("{}", message)?;
        }
        Ok(())
    }

    /// Print the given message to stderr, in red if possible
    pub fn print_error(&self, message: &str) -> buck2_error::Result<()> {
        self.stderr_colored_ln(message, Color::DarkRed)
    }

    /// Print the given message to stderr, in yellow if possible
    pub fn print_warning(&self, message: &str) -> buck2_error::Result<()> {
        self.stderr_colored_ln(message, Color::Yellow)
    }

    /// Print the given message to stderr, in green if possible
    pub fn print_success(&self, message: &str) -> buck2_error::Result<()> {
        self.stderr_colored_ln(message, Color::Green)
    }

    /// Print the given message to stderr, in green if possible
    pub fn print_success_no_newline(&self, message: &str) -> buck2_error::Result<()> {
        self.stderr_colored(message, Color::Green)
    }

    /// Print a string directly to stderr with no extra formatting
    pub fn print_stderr(&self, message: &str) -> buck2_error::Result<()> {
        crate::eprintln!("{}", message)
    }

    pub fn is_tty(&self) -> bool {
        self.is_tty
    }

    /// Returns true if the terminal is likely to support OSC 8 hyperlinks.
    /// Detection is based on environment variables set by known terminals.
    pub fn supports_hyperlinks(&self) -> bool {
        self.is_tty && terminal_supports_hyperlinks()
    }
}

/// Returns true if the terminal is likely to support OSC 8 hyperlinks,
/// based on environment variables set by known terminals.
/// This does not check whether stderr is a TTY; callers should check that separately
/// if needed.
///
/// Known terminals and their detection:
///
/// ```text
/// Support  Terminal          $TERM              Env Var
/// -------  ----------------  -----------------  -------------------------
/// Yes      VSCode terminal   xterm-256color     TERM_PROGRAM=vscode
/// Yes      iTerm2            xterm-256color     ITERM_SESSION_ID
/// Yes      Kitty             xterm-kitty        KITTY_WINDOW_ID
/// Yes      WezTerm           wezterm            WEZTERM_EXECUTABLE
/// Yes      GNOME Terminal    xterm-256color     VTE_VERSION
/// Yes      Konsole           xterm-256color     (none)
/// Yes      Windows Terminal  xterm-256color     WT_SESSION
/// Yes      Alacritty (Win)   xterm-256color     OS=Windows_NT / ComSpec
/// No       xterm             xterm              (none)
/// No       rxvt              rxvt               (none)
/// No       Terminal.app      xterm-256color     (none)
/// ```
///
/// NOTE: This is not a perfect function. SSH masks important env vars
///       needed for accurate detection. Thus iTerm2, WezTerm, etc.
///       over SSH may not be detected because they are indistinguishable
///       from Apple Terminal.app. We cater to the lowest common
///       denominator in ambiguous cases.
fn terminal_supports_hyperlinks() -> bool {
    // Known hyperlink-capable terminals detected via their env vars:
    // iTerm2, Kitty, WezTerm, GNOME Terminal (VTE), Windows Terminal
    if std::env::var_os("ITERM_SESSION_ID").is_some()
        || std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var_os("WEZTERM_EXECUTABLE").is_some()
        || std::env::var_os("VTE_VERSION").is_some()
        || std::env::var_os("WT_SESSION").is_some()
        || std::env::var_os("TERM_PROGRAM").is_some_and(|term| term == "vscode")
    {
        return true;
    }
    if std::env::var_os("TERM").is_some_and(|term| term == "xterm-256color") {
        // Local Windows shells (e.g. Alacritty, Windows Terminal without
        // WT_SESSION) reliably set OS=Windows_NT and ComSpec. These terminals
        // support hyperlinks.
        if std::env::var_os("OS").is_some_and(|v| v == "Windows_NT")
            || std::env::var_os("ComSpec").is_some()
        {
            return true;
        }
        // SSH sessions from Windows (e.g. Alacritty) may set SKS_PLATFORM.
        if std::env::var_os("SKS_PLATFORM").is_some_and(|v| v == "WINDOWS") {
            return true;
        }
        // SSH without distinguishing env vars — could be Terminal.app which
        // does NOT support hyperlinks. Cater to lowest common denominator.
        if std::env::var_os("SSH_TTY").is_some()
            || std::env::var_os("TERM_PROGRAM").is_some_and(|term| term == "Apple_Terminal")
        {
            return false;
        }
        // Konsole or other local xterm-256color terminal
        return true;
    }
    false
}
