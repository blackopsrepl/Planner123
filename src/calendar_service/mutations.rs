pub fn create_calendar(
    conn: &Connection,
    input: CreateCalendarInput,
) -> Result<Calendar, CalendarServiceError> {
    let source = input.source;
    let google_id = normalize_optional(input.google_id);
    validate_calendar_source(&source, google_id.as_deref())?;
    if let Some(google_id) = google_id.as_deref() {
        ensure_google_calendar_not_imported(conn, google_id, None)?;
    }
    let position = match input.position {
        Some(position) => position,
        None => db::next_calendar_position(conn).map_err(map_internal)?,
    };

    let now = timestamp_now();
    let calendar = Calendar {
        id: Uuid::new_v4().to_string(),
        name: normalize_required(input.name, "name")?,
        color: normalize_required(input.color, "color")?,
        source,
        google_id,
        visible: input.visible,
        position,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    };

    db::insert_calendar(conn, &calendar)
        .map_err(|err| map_write_error(err, calendar.google_id.as_deref()))?;
    Ok(calendar)
}

pub fn update_calendar(
    conn: &Connection,
    input: UpdateCalendarInput,
) -> Result<Calendar, CalendarServiceError> {
    let tx = conn.unchecked_transaction().map_err(map_internal)?;
    let mut calendar = db::get_calendar(&tx, &input.id)
        .map_err(map_internal)?
        .ok_or_else(|| CalendarServiceError::NotFound {
            resource: "calendar",
            id: input.id.clone(),
        })?;
    let original_source = calendar.source.clone();
    let original_google_id = calendar.google_id.clone();

    if let Some(name) = input.name {
        calendar.name = normalize_required(name, "name")?;
    }
    if let Some(color) = input.color {
        calendar.color = normalize_required(color, "color")?;
    }
    if let Some(source) = input.source {
        calendar.source = source;
    }
    if let Some(visible) = input.visible {
        calendar.visible = visible;
    }
    if let Some(position) = input.position {
        calendar.position = position;
    }
    if let Some(google_id) = input.google_id {
        calendar.google_id = normalize_optional(google_id);
    }
    if calendar.source == CalendarSource::Local {
        calendar.google_id = None;
    }

    if original_source == CalendarSource::Google
        && calendar.source == CalendarSource::Google
        && original_google_id != calendar.google_id
    {
        return Err(CalendarServiceError::Validation(
            "changing google_id on an existing google calendar is not supported".to_string(),
        ));
    }

    validate_calendar_source(&calendar.source, calendar.google_id.as_deref())?;
    if let Some(google_id) = calendar.google_id.as_deref() {
        ensure_google_calendar_not_imported(&tx, google_id, Some(&calendar.id))?;
    }

    db::update_calendar(&tx, &calendar)
        .map_err(|err| map_write_error(err, calendar.google_id.as_deref()))?;
    if original_source == CalendarSource::Google && calendar.source == CalendarSource::Local {
        db::detach_google_sync_state_for_calendar(&tx, &calendar.id).map_err(map_internal)?;
    }

    let updated = db::get_calendar(&tx, &calendar.id)
        .map_err(map_internal)?
        .ok_or_else(|| {
            CalendarServiceError::Internal("calendar disappeared after update".into())
        })?;
    tx.commit().map_err(map_internal)?;
    Ok(updated)
}

pub fn import_google_calendar(
    conn: &Connection,
    calendar: &DiscoveredGoogleCalendar,
    position: Option<i64>,
) -> Result<Calendar, CalendarServiceError> {
    let tx = conn.unchecked_transaction().map_err(map_internal)?;
    let imported = create_calendar(
        &tx,
        CreateCalendarInput {
            name: calendar.name.clone(),
            color: calendar.color.clone(),
            source: CalendarSource::Google,
            google_id: Some(calendar.google_id.clone()),
            visible: true,
            position,
        },
    )?;
    crate::sync::state::upsert_calendar_sync_state(
        &tx,
        &CalendarSyncState {
            calendar_id: imported.id.clone(),
            google_access_role: calendar.access_role.clone(),
            writable: calendar.writable,
            last_synced_at: None,
            last_sync_error_code: None,
            last_sync_error_message: None,
            pending_outbox: 0,
            pending_conflicts: 0,
            created_at: String::new(),
            updated_at: String::new(),
        },
    )
    .map_err(map_internal)?;
    tx.commit().map_err(map_internal)?;
    Ok(imported)
}

pub fn filter_unimported_google_calendars(
    conn: &Connection,
    discovered: Vec<DiscoveredGoogleCalendar>,
) -> Result<Vec<DiscoveredGoogleCalendar>, CalendarServiceError> {
    let imported_google_ids = db::load_calendars(conn)
        .map_err(map_internal)?
        .into_iter()
        .filter(|calendar| calendar.source == CalendarSource::Google)
        .filter_map(|calendar| calendar.google_id)
        .collect::<HashSet<_>>();

    Ok(discovered
        .into_iter()
        .filter(|calendar| !imported_google_ids.contains(&calendar.google_id))
        .collect())
}
