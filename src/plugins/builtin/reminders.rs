// src/plugins/builtin/reminders.rs
// Alarm and Reminder plugin

use super::*;

pub fn plugin() -> Plugin {
    Plugin {
        metadata: PluginMetadata {
            name: "reminders".to_string(),
            version: "1.0.0".to_string(),
            author: "IGRIS".to_string(),
            description: "Set alarms and reminders with voice commands".to_string(),
            keywords: vec![
                "alarm", "reminder", "remind", "alert", "notify", "notification",
                "timer", "schedule", "wake", "clock"
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            enabled: true,
        },
        commands: vec![
            // Alarm commands
            cmd!(
                "set alarm",
                "Sets an alarm for a specific time",
                &[
                    "set alarm for 7 am",
                    "set alarm at 6:30 pm",
                    "wake me up at 8 am",
                    "alarm for 5:45 pm"
                ],
                ActionType::CustomFunction,
                "alarm_set"
            ),
            cmd!(
                "cancel alarm",
                "Cancels all active alarms",
                &["cancel alarm", "stop alarm", "delete alarm", "remove alarm"],
                ActionType::CustomFunction,
                "alarm_cancel"
            ),
            cmd!(
                "show alarms",
                "Shows all active alarms",
                &["show alarms", "list alarms", "what alarms", "my alarms"],
                ActionType::CustomFunction,
                "alarm_list"
            ),
            
            // Reminder commands
            cmd!(
                "remind me",
                "Sets a reminder for later",
                &[
                    "remind me to call mom in 30 minutes",
                    "remind me about meeting in 2 hours",
                    "set reminder for 5 pm to buy groceries",
                    "reminder in 10 minutes"
                ],
                ActionType::CustomFunction,
                "reminder_set"
            ),
            cmd!(
                "cancel reminder",
                "Cancels all active reminders",
                &["cancel reminder", "stop reminder", "delete reminder", "remove reminder"],
                ActionType::CustomFunction,
                "reminder_cancel"
            ),
            cmd!(
                "show reminders",
                "Shows all active reminders",
                &["show reminders", "list reminders", "what reminders", "my reminders"],
                ActionType::CustomFunction,
                "reminder_list"
            ),
        ],
    }
}
