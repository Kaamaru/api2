use chrono::{DateTime, Local, TimeZone, Utc};
use chrono_tz::Asia::Bangkok; // GMT+7 timezone
use tokio::time::{sleep, Duration, Instant};

use crate::tasks::sendmail::do_expiration_mail;

// Target time configuration
const TARGET_HOUR: u32 = 21;
const TARGET_MINUTE: u32 = 52;
const TARGET_SECOND: u32 = 30;


pub async fn timeloop() {
    println!(
        "Starting GMT+7 {}:{:02}:{:02} hello timer...",
        TARGET_HOUR, TARGET_MINUTE, TARGET_SECOND
    );

    loop {
        match wait_for_next_target_time().await {
            Ok(()) => {
                println!("Sending mails");
                tokio::spawn(async { do_expiration_mail().await });
                sleep(Duration::from_secs(61)).await;
            }
            Err(e) => {
                eprintln!("Error calculating next target time: {}", e);
                sleep(Duration::from_secs(300)).await;
            }
        }
    }
}

pub async fn wait_for_next_target_time() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now = Bangkok.from_utc_datetime(&Utc::now().naive_utc());

    // Calculate next target time in GMT+7
    let target_time_today = now
        .date_naive()
        .and_hms_opt(TARGET_HOUR, TARGET_MINUTE, TARGET_SECOND)
        .ok_or("Failed to create target time")?;

    let next_target = if now.time() < target_time_today.time() {
        // Today's target time hasn't passed yet
        target_time_today
    } else {
        // Today's target time has passed, get tomorrow's
        (now.date_naive() + chrono::Duration::days(1))
            .and_hms_opt(TARGET_HOUR, TARGET_MINUTE, TARGET_SECOND)
            .ok_or("Failed to create tomorrow's target time")?
    };

    let next_target_gmt7 = Bangkok
        .from_local_datetime(&next_target)
        .single()
        .ok_or("Failed to convert to GMT+7")?;

    let duration_until_target = next_target_gmt7.signed_duration_since(now);

    if duration_until_target.num_seconds() <= 0 {
        return Err("Calculated time is in the past".into());
    }

    let wait_duration = Duration::from_secs(duration_until_target.num_seconds() as u64);
    let hours = duration_until_target.num_hours();
    let minutes = (duration_until_target.num_minutes() % 60);
    let seconds = (duration_until_target.num_seconds() % 60);

    println!(
        "Current time (GMT+7): {}",
        now.format("%Y-%m-%d %H:%M:%S %Z")
    );
    println!(
        "Next target time (GMT+7): {}",
        next_target_gmt7.format("%Y-%m-%d %H:%M:%S %Z")
    );
    println!("Waiting: {}h {}m {}s", hours, minutes, seconds);

    // Use tokio's sleep with periodic check every 10 minutes
    let start_time = Instant::now();
    let mut remaining = wait_duration;

    while remaining > Duration::from_secs(0) {
        let check_interval = Duration::from_secs(600); // 10 minutes
        let sleep_duration = if remaining > check_interval {
            check_interval
        } else {
            remaining
        };

        sleep(sleep_duration).await;

        let elapsed = start_time.elapsed();
        if elapsed >= wait_duration {
            break;
        }

        remaining = wait_duration - elapsed;

        // Periodic status update
        if remaining > Duration::from_secs(60) {
            let remaining_minutes = remaining.as_secs() / 60;
            println!("Still waiting... {} minutes remaining", remaining_minutes);
        }
    }

    Ok(())
}
