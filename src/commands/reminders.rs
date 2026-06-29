// src/commands/reminders.rs
// Alarm and Reminder command handler with background scheduler

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use chrono::{DateTime, Local, Duration, Timelike};
use once_cell::sync::Lazy;
use std::thread;
use std::time::Duration as StdDuration;
use crate::nlu::ner::GLOBAL_NER;

#[derive(Debug, Clone)]
pub struct Alarm {
    pub id: u32,
    pub time: DateTime<Local>,
    pub message: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: u32,
    pub trigger_time: DateTime<Local>,
    pub message: String,
    pub active: bool,
}

pub struct ReminderManager {
    alarms: Arc<Mutex<HashMap<u32, Alarm>>>,
    reminders: Arc<Mutex<HashMap<u32, Reminder>>>,
    next_id: Arc<Mutex<u32>>,
}

impl ReminderManager {
    pub fn new() -> Self {
        Self {
            alarms: Arc::new(Mutex::new(HashMap::new())),
            reminders: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }
    
    fn get_next_id(&self) -> u32 {
        let mut id = self.next_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current
    }
    
    pub fn add_alarm(&self, time: DateTime<Local>, message: String) -> u32 {
        let id = self.get_next_id();
        let alarm = Alarm {
            id,
            time,
            message,
            active: true,
        };
        
        self.alarms.lock().unwrap().insert(id, alarm);
        println!("[REMINDER] Alarm {} set for {}", id, time.format("%I:%M %p"));
        id
    }
    
    pub fn add_reminder(&self, trigger_time: DateTime<Local>, message: String) -> u32 {
        let id = self.get_next_id();
        let reminder = Reminder {
            id,
            trigger_time,
            message,
            active: true,
        };
        
        self.reminders.lock().unwrap().insert(id, reminder);
        println!("[REMINDER] Reminder {} set for {}", id, trigger_time.format("%I:%M %p"));
        id
    }
    
    pub fn cancel_all_alarms(&self) -> usize {
        let mut alarms = self.alarms.lock().unwrap();
        let count = alarms.len();
        alarms.clear();
        count
    }
    
    pub fn cancel_all_reminders(&self) -> usize {
        let mut reminders = self.reminders.lock().unwrap();
        let count = reminders.len();
        reminders.clear();
        count
    }
    
    pub fn list_alarms(&self) -> Vec<Alarm> {
        self.alarms.lock().unwrap().values().cloned().collect()
    }
    
    pub fn list_reminders(&self) -> Vec<Reminder> {
        self.reminders.lock().unwrap().values().cloned().collect()
    }
    
    pub fn check_and_trigger(&self) {
        let now = Local::now();
        
        // Check alarms
        let mut alarms = self.alarms.lock().unwrap();
        let mut to_remove = Vec::new();
        
        for (id, alarm) in alarms.iter() {
            if alarm.active && now >= alarm.time {
                println!("[REMINDER] 🔔 ALARM TRIGGERED: {}", alarm.message);
                
                // Trigger TTS
                let msg = format!("Alarm! {}", alarm.message);
                let _ = crate::core::tts::speak(&msg);
                
                to_remove.push(*id);
            }
        }
        
        for id in to_remove {
            alarms.remove(&id);
        }
        drop(alarms);
        
        // Check reminders
        let mut reminders = self.reminders.lock().unwrap();
        let mut to_remove = Vec::new();
        
        for (id, reminder) in reminders.iter() {
            if reminder.active && now >= reminder.trigger_time {
                println!("[REMINDER] 📌 REMINDER TRIGGERED: {}", reminder.message);
                
                // Trigger TTS
                let msg = format!("Reminder: {}", reminder.message);
                let _ = crate::core::tts::speak(&msg);
                
                to_remove.push(*id);
            }
        }
        
        for id in to_remove {
            reminders.remove(&id);
        }
    }
}

pub static REMINDER_MANAGER: Lazy<ReminderManager> = Lazy::new(|| {
    let manager = ReminderManager::new();
    
    // Start background checker thread
    let alarms = manager.alarms.clone();
    let reminders = manager.reminders.clone();
    
    thread::spawn(move || {
        let temp_manager = ReminderManager {
            alarms,
            reminders,
            next_id: Arc::new(Mutex::new(0)),
        };
        
        loop {
            thread::sleep(StdDuration::from_secs(10)); // Check every 10 seconds
            temp_manager.check_and_trigger();
        }
    });
    
    manager
});

/// Parse time from command (e.g., "7 am", "6:30 pm", "18:45")
fn parse_time(command: &str) -> Option<DateTime<Local>> {
    let cmd_lower = command.to_lowercase();
    
    // Try to extract time patterns
    let time_patterns = [
        // "7 am", "7am", "7 a.m."
        (r"(\d{1,2})\s*(?:am|a\.m\.)", true),
        // "7 pm", "7pm", "7 p.m."
        (r"(\d{1,2})\s*(?:pm|p\.m\.)", false),
        // "7:30 am", "7:30am"
        (r"(\d{1,2}):(\d{2})\s*(?:am|a\.m\.)", true),
        // "7:30 pm", "7:30pm"
        (r"(\d{1,2}):(\d{2})\s*(?:pm|p\.m\.)", false),
    ];
    
    for (pattern, is_am) in &time_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(caps) = re.captures(&cmd_lower) {
                let hour: u32 = caps.get(1)?.as_str().parse().ok()?;
                let minute: u32 = caps.get(2).map(|m| m.as_str().parse().ok()).flatten().unwrap_or(0);
                
                let mut hour24 = hour;
                if !is_am && hour != 12 {
                    hour24 += 12;
                } else if *is_am && hour == 12 {
                    hour24 = 0;
                }
                
                let mut target = Local::now()
                    .with_hour(hour24)?
                    .with_minute(minute)?
                    .with_second(0)?
                    .with_nanosecond(0)?;
                
                // If time has passed today, set for tomorrow
                if target <= Local::now() {
                    target = target + Duration::days(1);
                }
                
                return Some(target);
            }
        }
    }
    
    None
}

/// Parse duration from command (e.g., "in 30 minutes", "in 2 hours")
fn parse_duration(command: &str) -> Option<DateTime<Local>> {
    let duration_secs = GLOBAL_NER.parse_duration_seconds(command)?;
    Some(Local::now() + Duration::seconds(duration_secs as i64))
}

/// Handle alarm commands
pub fn handle_alarm_command(action: &str, command: &str) -> Result<String, String> {
    match action {
        "alarm_set" => {
            if let Some(time) = parse_time(command) {
                let message = "Time to wake up!".to_string();
                let id = REMINDER_MANAGER.add_alarm(time, message);
                Ok(format!("Alarm set for {}", time.format("%I:%M %p")))
            } else {
                Err("Could not understand the time. Try 'set alarm for 7 am'".to_string())
            }
        }
        
        "alarm_cancel" => {
            let count = REMINDER_MANAGER.cancel_all_alarms();
            if count > 0 {
                Ok(format!("Cancelled {} alarm{}", count, if count == 1 { "" } else { "s" }))
            } else {
                Ok("No active alarms to cancel".to_string())
            }
        }
        
        "alarm_list" => {
            let alarms = REMINDER_MANAGER.list_alarms();
            if alarms.is_empty() {
                Ok("No active alarms".to_string())
            } else {
                let list: Vec<String> = alarms.iter()
                    .map(|a| format!("Alarm at {}", a.time.format("%I:%M %p")))
                    .collect();
                Ok(format!("Active alarms: {}", list.join(", ")))
            }
        }
        
        _ => Err(format!("Unknown alarm action: {}", action))
    }
}

/// Handle reminder commands
pub fn handle_reminder_command(action: &str, command: &str) -> Result<String, String> {
    match action {
        "reminder_set" => {
            // Try duration first (e.g., "in 30 minutes")
            if let Some(trigger_time) = parse_duration(command) {
                // Extract message after "to" or "about"
                let message = if let Some(pos) = command.find(" to ") {
                    command[pos + 4..].trim().to_string()
                } else if let Some(pos) = command.find(" about ") {
                    command[pos + 7..].trim().to_string()
                } else {
                    "Reminder".to_string()
                };
                
                let id = REMINDER_MANAGER.add_reminder(trigger_time, message.clone());
                let duration_str = if trigger_time.signed_duration_since(Local::now()).num_minutes() < 60 {
                    format!("{} minutes", trigger_time.signed_duration_since(Local::now()).num_minutes())
                } else {
                    format!("{} hours", trigger_time.signed_duration_since(Local::now()).num_hours())
                };
                Ok(format!("Reminder set for {}", duration_str))
            }
            // Try specific time (e.g., "at 5 pm")
            else if let Some(trigger_time) = parse_time(command) {
                let message = if let Some(pos) = command.find(" to ") {
                    command[pos + 4..].trim().to_string()
                } else if let Some(pos) = command.find(" about ") {
                    command[pos + 7..].trim().to_string()
                } else {
                    "Reminder".to_string()
                };
                
                let id = REMINDER_MANAGER.add_reminder(trigger_time, message.clone());
                Ok(format!("Reminder set for {}", trigger_time.format("%I:%M %p")))
            } else {
                Err("Could not understand the time. Try 'remind me in 30 minutes' or 'remind me at 5 pm'".to_string())
            }
        }
        
        "reminder_cancel" => {
            let count = REMINDER_MANAGER.cancel_all_reminders();
            if count > 0 {
                Ok(format!("Cancelled {} reminder{}", count, if count == 1 { "" } else { "s" }))
            } else {
                Ok("No active reminders to cancel".to_string())
            }
        }
        
        "reminder_list" => {
            let reminders = REMINDER_MANAGER.list_reminders();
            if reminders.is_empty() {
                Ok("No active reminders".to_string())
            } else {
                let list: Vec<String> = reminders.iter()
                    .map(|r| format!("{} at {}", r.message, r.trigger_time.format("%I:%M %p")))
                    .collect();
                Ok(format!("Active reminders: {}", list.join(", ")))
            }
        }
        
        _ => Err(format!("Unknown reminder action: {}", action))
    }
}
