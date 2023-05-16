use anyhow::Context as _;
use libcommand::{impl_command, CommandClient, TrinityCommand};
use regex::Regex;
use serde::{Deserialize, Serialize};
use shlex;
use std::collections::HashSet;
use textwrap_macros::dedent;

use wit_log as log;

#[derive(Debug, Default, Deserialize, Serialize)]
struct Rule {
    name: String,
    re: String,
    sub: String,
}

impl TryFrom<&str> for Rule {
    type Error = String;

    fn try_from(cmd: &str) -> anyhow::Result<Self, Self::Error> {
        let words = shlex::split(&cmd).unwrap_or_default();
        if words.len() != 3 {
            return Err(String::from("Three inputs expected: name/re/sub"));
        }

        Ok(Self {
            name: words[0].clone(),
            re: words[1].clone(),
            sub: words[2].clone(),
        })
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct RoomConfig {
    enabled_rules: HashSet<String>,
}

struct Component;

impl Component {
    fn get_rules() -> Vec<Rule> {
        wit_kv::get::<_, Vec<Rule>>("rules")
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    fn get_room_config(room: &str) -> RoomConfig {
        wit_kv::get::<_, RoomConfig>(&format!("room:{}", room))
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    fn replace(msg: &str, rc: RoomConfig) -> Option<String> {
        for rule in Self::get_rules()
            .iter()
            .filter(|&r| rc.enabled_rules.contains(&r.name))
        {
            let Ok(re) = Regex::new(&rule.re) else {
                // Shouldn't happen in theory, since the rules are validated at creation.
                log::warn!("unexpected invalid regex in replace()");
                continue;
            };
            if let Some(caps) = re.captures(msg) {
                let mut dest = String::new();
                caps.expand(&rule.sub, &mut dest);
                return Some(dest);
            }
        }
        None
    }

    fn handle_admin(cmd: &str, _sender: &str, room: &str) -> anyhow::Result<String> {
        // Format: new NAME RE SUB
        if let Some(input) = cmd.strip_prefix("new") {
            let rule = match Rule::try_from(input) {
                Ok(r) => r,
                Err(e) => return Ok(format!("Error parsing rule: {}", e)),
            };

            let mut rules = Self::get_rules();

            // Don't overwrite existing rules
            if rules.iter().any(|r| r.name == rule.name) {
                return Ok(format!("Rule '{}' already exists!", &rule.name));
            }

            // Ensure the regex is valid
            if Regex::new(&rule.re).is_err() {
                return Ok(format!("Invalid regex `{}`!", &rule.re));
            }

            rules.push(rule);
            let _ = wit_kv::set("rules", &rules);
            return Ok("Rule has been created!".into());
        }

        // Format: delete NAME
        if let Some(cmd) = cmd.strip_prefix("delete") {
            let mut split = cmd.trim().split_whitespace();
            let name = split.next().context("missing name")?;
            let mut rules = Self::get_rules();
            if let Some(index) = rules.iter().position(|r| r.name == name) {
                rules.remove(index);
                let _ = wit_kv::set("rules", &rules);
                return Ok("Rule has been deleted!".into());
            }
            return Ok(format!("Rule '{}' not found!", &name));
        }

        // Format: enable NAME
        if let Some(cmd) = cmd.strip_prefix("enable") {
            let mut split = cmd.trim().split_whitespace();
            let name = split.next().context("missing name")?;
            let rules = Self::get_rules();
            if rules.iter().any(|r| r.name == name) {
                let mut rc = Self::get_room_config(room);
                if rc.enabled_rules.contains(name) {
                    return Ok(format!("Rule '{}' is already enabled!", &name));
                }
                rc.enabled_rules.insert(name.to_string());
                let _ = wit_kv::set(&format!("room:{}", &room), &rc);
                return Ok(format!("Rule '{}' has been enabled!", &name));
            }
            return Ok(format!("Rule '{}' not found!", &name));
        }

        // Format: disable NAME
        if let Some(cmd) = cmd.strip_prefix("disable") {
            let mut split = cmd.trim().split_whitespace();
            let name = split.next().context("missing name")?;
            let rules = Self::get_rules();
            if rules.iter().any(|r| r.name == name) {
                let mut rc = Self::get_room_config(room);
                if rc.enabled_rules.remove(name) {
                    let _ = wit_kv::set(&format!("room:{}", &room), &rc);
                    return Ok(format!("Rule '{}' has been disabled!", &name));
                }
                return Ok(format!("Rule '{}' is already disabled!", &name));
            }
            return Ok(format!("Rule '{}' not found!", &name));
        }

        // Format: list
        if cmd == "list" {
            let rules = Self::get_rules();
            if rules.is_empty() {
                return Ok("No rules found.".to_string());
            }
            let mut msg = String::new();
            for rule in rules {
                msg.push_str(&format!("\n{:?}", &rule));
            }
            return Ok(msg);
        }

        Ok(format!(
            "Unknown command '{}'!",
            cmd.split_whitespace().next().unwrap_or("none")
        ))
    }
}

impl TrinityCommand for Component {
    fn init() {
        let _ = log::set_boxed_logger(Box::new(log::WitLog::new()));
        log::set_max_level(log::LevelFilter::Trace);
        log::trace!("Called the init() method \\o/");
    }

    fn on_help(topic: Option<&str>) -> String {
        if let Some(topic) = topic {
            match topic {
                "admin" => dedent!(
                    r#"
                    ### Command Overview

                    Available admin commands:

                    - new #NAME #RE #SUB
                    - delete #NAME
                    - enable #NAME
                    - disable #NAME
                    - list

                    ### Creating and Enabling Rules

                    Rules must first be created using the `new` command, then enabled on a
                    room by room basis. To create a rule, run
                    `!admin linkify new <name> <regex> <substitution>`. Where `<name>` is an
                    identifier to refer to the rule for future use, `<regex>` is a regular expression
                    to match on text, and `<substitution>` is a string into which the regex capture
                    groups can be interpolated.

                    For example, to create a new rule which links to a Github issue, run something
                    like:

                        !admin linkify new issue "(issue ?#?|# ?)([0-9]+)(\\s|$)" https://github.com/bnjbvr/trinity/issues/$2

                    The `$2` will be substituted with the second regex capture group (which is the
                    issue number in this example). It's also possible to use named capture groups, e.g:

                        !admin linkify new issue "(issue ?#?|# ?)(?P<issue>[0-9]+)(\\s|$)" https://github.com/bnjbvr/trinity/issues/${issue}

                    Then for each room you'd like this rule enabled, run:

                        !admin linkify enable issue

                    Now anytime someone types a string like issue 123 or #123, linkify will respond
                    with the appropriate URL.

                    The `disable` and `delete` commands take a single `<name>` argument and will
                    disable the rule, or delete it globally respectively.

            "#
                )
                .into(),
                _ => "Invalid command!".into(),
            }
        } else {
            "Create regex based substition rules per channel! Help topics: admin".to_owned()
        }
    }

    fn on_msg(client: &mut CommandClient, content: &str) {
        if let Some(content) = Self::replace(&content, Self::get_room_config(client.room())) {
            client.respond(content);
        }
    }

    fn on_admin(client: &mut CommandClient, cmd: &str) {
        let content = match Self::handle_admin(&cmd, client.from(), client.room()) {
            Ok(resp) => resp,
            Err(err) => err.to_string(),
        };
        client.respond(content);
    }
}

impl_command!(Component);
