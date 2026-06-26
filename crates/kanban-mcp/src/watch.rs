mod args;
mod delivery;
mod guard;

pub(crate) use args::{usage, WatchArgs};

use anyhow::Result;
use kanban_core::{HandoffStatusPatch, Store};
use std::time::Duration;

use self::delivery::deliver;
use self::guard::WatchGuard;

pub(crate) fn run(store: &mut Store, args: WatchArgs) -> Result<()> {
    let Some(guard) = WatchGuard::claim(
        &args.run_dir,
        &args.for_agent,
        args.replace_existing,
        args.skip_if_running,
    )?
    else {
        return Ok(());
    };
    loop {
        let delivered = scan_once(store, &args)?;
        guard.mark_ready()?;
        if args.once {
            return Ok(());
        }
        if delivered == 0 {
            std::thread::sleep(Duration::from_millis(args.interval_ms));
        }
    }
}

fn scan_once(store: &mut Store, args: &WatchArgs) -> Result<usize> {
    let handoffs = store.list_handoffs(Some(&args.for_agent), false, 100)?;
    let mut delivered = 0;
    for handoff in handoffs {
        let claimed = match store.claim_handoff(
            &handoff.id,
            &args.for_agent,
            Some(&args.claim_token),
            args.lease_minutes,
        ) {
            Ok(claimed) => claimed,
            Err(_) => continue,
        };
        match deliver(&claimed, args.bridge.as_ref()) {
            Ok(()) => {
                store.update_handoff_status(
                    &claimed.id,
                    &args.for_agent,
                    Some(&args.claim_token),
                    &HandoffStatusPatch {
                        status: "completed".into(),
                        note: Some("delivered by watch-handoffs".into()),
                    },
                )?;
                delivered += 1;
            }
            Err(err) => {
                store.update_handoff_status(
                    &claimed.id,
                    &args.for_agent,
                    Some(&args.claim_token),
                    &HandoffStatusPatch {
                        status: "failed".into(),
                        note: Some(err.to_string()),
                    },
                )?;
                return Err(err);
            }
        }
    }
    Ok(delivered)
}
