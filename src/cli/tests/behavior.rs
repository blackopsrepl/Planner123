use super::*;

#[test]
fn blocks_cycle_is_rejected() {
    let (_temp, conn) = open_test_db();
    let calendar = seed_calendar(&conn, "Work");
    let event_a = seed_event(&conn, &calendar.id, None, "A");
    let event_b = seed_event(&conn, &calendar.id, None, "B");

    let first = Cli {
        command: Command::Dependencies {
            action: DependencyCommand::Create(DependencyCreateArgs {
                from_event_id: event_a.id.clone(),
                to_event_id: event_b.id.clone(),
                dependency_type: DependencyTypeArg::Blocks,
            }),
        },
    };
    execute_with_connection(&conn, first).unwrap();

    let second = Cli {
        command: Command::Dependencies {
            action: DependencyCommand::Create(DependencyCreateArgs {
                from_event_id: event_b.id.clone(),
                to_event_id: event_a.id.clone(),
                dependency_type: DependencyTypeArg::Blocks,
            }),
        },
    };
    let err = execute_with_connection(&conn, second).unwrap_err();
    assert_eq!(err.code, "conflict");
}

#[test]
fn project_delete_requires_detach_flag() {
    let (_temp, conn) = open_test_db();
    let calendar = seed_calendar(&conn, "Work");
    let project = seed_project(&conn, "Launch");
    seed_event(&conn, &calendar.id, Some(project.id.clone()), "Milestone");

    let cli = Cli {
        command: Command::Projects {
            action: ProjectCommand::Delete(ProjectDeleteArgs {
                id: project.id.clone(),
                detach_events: false,
            }),
        },
    };
    let err = execute_with_connection(&conn, cli).unwrap_err();
    assert_eq!(err.code, "conflict");
}

#[test]
fn calendar_delete_rejects_last_active_calendar() {
    let (_temp, conn) = open_test_db();
    let only_calendar = db::load_calendars(&conn)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    let cli = Cli {
        command: Command::Calendars {
            action: CalendarCommand::Delete(CalendarDeleteArgs {
                id: only_calendar.id,
                cascade_events: false,
            }),
        },
    };
    let err = execute_with_connection(&conn, cli).unwrap_err();
    assert_eq!(err.code, "conflict");
    assert_eq!(err.message, "cannot delete the last active calendar");
}

#[test]
fn project_delete_with_detach_clears_project_id() {
    let (_temp, conn) = open_test_db();
    let calendar = seed_calendar(&conn, "Work");
    let project = seed_project(&conn, "Launch");
    let event = seed_event(&conn, &calendar.id, Some(project.id.clone()), "Milestone");

    let cli = Cli {
        command: Command::Projects {
            action: ProjectCommand::Delete(ProjectDeleteArgs {
                id: project.id.clone(),
                detach_events: true,
            }),
        },
    };
    execute_with_connection(&conn, cli).unwrap();

    let updated = db::get_event(&conn, &event.id).unwrap().unwrap();
    assert_eq!(updated.project_id, None);
    assert!(db::get_project(&conn, &project.id).unwrap().is_none());
}

#[test]
fn event_update_conflicting_flags_are_rejected() {
    let (_temp, conn) = open_test_db();
    let calendar = seed_calendar(&conn, "Work");
    let event = seed_event(&conn, &calendar.id, None, "Milestone");

    let cli = Cli {
        command: Command::Events {
            action: EventCommand::Update(EventUpdateArgs {
                id: event.id,
                calendar_id: None,
                title: None,
                project_id: None,
                clear_project_id: false,
                description: Some("new".to_string()),
                clear_description: true,
                location: None,
                clear_location: false,
                start_at: None,
                end_at: None,
                all_day: None,
                rrule: None,
                clear_rrule: false,
                reminder_minutes: None,
                clear_reminder_minutes: false,
                timezone: None,
            }),
        },
    };

    let err = execute_with_connection(&conn, cli).unwrap_err();
    assert_eq!(err.code, "validation_error");
}
