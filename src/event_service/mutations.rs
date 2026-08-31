pub fn save_event(
    conn: &Connection,
    event: Event,
    is_new: bool,
) -> Result<Event, EventServiceError> {
    let tx = conn.unchecked_transaction().map_err(map_internal)?;
    let saved = save_event_in_transaction(&tx, event, is_new)?;
    tx.commit().map_err(map_internal)?;
    Ok(saved)
}

pub(crate) fn save_event_in_transaction(
    conn: &Connection,
    mut event: Event,
    is_new: bool,
) -> Result<Event, EventServiceError> {
    let previous_event = if is_new {
        None
    } else {
        Some(
            db::get_event(conn, &event.id)
                .map_err(map_internal)?
                .ok_or_else(|| EventServiceError::NotFound {
                    resource: "event",
                    id: event.id.clone(),
                })?,
        )
    };
    let calendar = prepare_event(conn, &mut event, is_new)?;

    if is_new {
        db::insert_event(conn, &event).map_err(map_internal)?;
    } else {
        db::update_event(conn, &event).map_err(map_internal)?;
    }
    state::record_local_event_save(conn, &calendar, previous_event.as_ref(), &event)
        .map_err(map_sync_error)?;

    db::get_event(conn, &event.id)
        .map_err(map_internal)?
        .ok_or_else(|| EventServiceError::Internal("event disappeared after save".to_string()))
}

pub fn delete_event(conn: &Connection, event_id: &str) -> Result<(), EventServiceError> {
    let tx = conn.unchecked_transaction().map_err(map_internal)?;
    delete_event_in_transaction(&tx, event_id)?;
    tx.commit().map_err(map_internal)?;
    Ok(())
}

pub(crate) fn delete_event_in_transaction(
    conn: &Connection,
    event_id: &str,
) -> Result<(), EventServiceError> {
    let event = db::get_event(conn, event_id)
        .map_err(map_internal)?
        .ok_or_else(|| EventServiceError::NotFound {
            resource: "event",
            id: event_id.to_string(),
        })?;
    let calendar = db::get_calendar(conn, &event.calendar_id)
        .map_err(map_internal)?
        .ok_or_else(|| EventServiceError::NotFound {
            resource: "calendar",
            id: event.calendar_id.clone(),
        })?;
    if calendar.source == CalendarSource::Google
        && !state::google_calendar_writable(conn, &calendar).map_err(map_sync_error)?
    {
        return Err(EventServiceError::Conflict(format!(
            "calendar '{}' is read-only and cannot accept local edits",
            calendar.name
        )));
    }
    state::record_local_event_delete(conn, &calendar, &event).map_err(map_sync_error)?;
    db::soft_delete_event(conn, event_id).map_err(map_internal)?;
    Ok(())
}
