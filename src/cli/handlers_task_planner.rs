fn handle_tasks(conn: &Connection, action: TaskCommand) -> Result<Value, CliError> {
    match action {
        TaskCommand::List => Ok(json!(planner::list_tasks(conn).map_err(planner_error)?)),
        TaskCommand::Get { id } => Ok(json!(planner::get_task(conn, &id)
            .map_err(planner_error)?
            .ok_or_else(|| CliError::not_found("task", &id))?)),
        TaskCommand::Create(args) => Ok(json!(planner::create_task(
            conn,
            planner::CreateTaskInput {
                title: args.title,
                duration_minutes: args.duration_minutes,
                target_calendar_id: args.target_calendar_id,
                project_id: normalize_optional(args.project_id),
                priority: task_priority_from_arg(args.priority),
                cognitive_load: cognitive_load_from_arg(args.cognitive_load),
                earliest_at: args.earliest_at,
                deadline_kind: deadline_kind_from_arg(args.deadline_kind),
                deadline_at: args.deadline_at,
            }
        )
        .map_err(planner_error)?)),
        TaskCommand::Update(args) => {
            if args.project_id.is_some() && args.clear_project_id {
                return Err(CliError::validation(
                    "cannot combine --project-id and --clear-project-id",
                ));
            }
            if args.earliest_at.is_some() && args.clear_earliest_at {
                return Err(CliError::validation(
                    "cannot combine --earliest-at and --clear-earliest-at",
                ));
            }
            if args.deadline_at.is_some() && args.clear_deadline_at {
                return Err(CliError::validation(
                    "cannot combine --deadline-at and --clear-deadline-at",
                ));
            }
            let project_id = if args.clear_project_id {
                Some(None)
            } else {
                args.project_id.map(Some)
            };
            let earliest_at = if args.clear_earliest_at {
                Some(None)
            } else {
                args.earliest_at.map(Some)
            };
            let deadline_at = if args.clear_deadline_at {
                Some(None)
            } else {
                args.deadline_at.map(Some)
            };
            Ok(json!(planner::update_task(
                conn,
                &args.id,
                planner::UpdateTaskInput {
                    title: args.title,
                    duration_minutes: args.duration_minutes,
                    target_calendar_id: args.target_calendar_id,
                    project_id,
                    priority: args.priority.map(task_priority_from_arg),
                    cognitive_load: args.cognitive_load.map(cognitive_load_from_arg),
                    earliest_at,
                    deadline_kind: args.deadline_kind.map(deadline_kind_from_arg),
                    deadline_at,
                }
            )
            .map_err(planner_error)?))
        }
        TaskCommand::Delete { id } => {
            planner::delete_task(conn, &id).map_err(planner_error)?;
            Ok(json!(DeleteData {
                resource: "task",
                id
            }))
        }
        TaskCommand::ReturnToInbox { id } => Ok(json!(
            planner::return_to_inbox(conn, &id).map_err(planner_error)?
        )),
        TaskCommand::Dependencies { action } => match action {
            TaskDependencyCommand::List => Ok(json!(
                planner::list_dependencies(conn).map_err(planner_error)?
            )),
            TaskDependencyCommand::Add(args) => {
                planner::add_dependency(conn, &args.from_task_id, &args.to_task_id)
                    .map_err(planner_error)?;
                Ok(json!({"from_task_id":args.from_task_id,"to_task_id":args.to_task_id}))
            }
            TaskDependencyCommand::Remove(args) => {
                planner::remove_dependency(conn, &args.from_task_id, &args.to_task_id)
                    .map_err(planner_error)?;
                Ok(json!(DeleteData {
                    resource: "task_dependency",
                    id: format!("{}:{}", args.from_task_id, args.to_task_id)
                }))
            }
        },
    }
}

fn handle_planner(conn: &Connection, action: PlannerCommand) -> Result<Value, CliError> {
    match action {
        PlannerCommand::Settings { action } => match action {
            PlannerSettingsCommand::Show => {
                Ok(planner::settings_json(conn).map_err(planner_error)?)
            }
            PlannerSettingsCommand::Update(args) => {
                let args = *args;
                let availability = parse_availability_args(args.availability)?;
                planner::update_settings(
                    conn,
                    planner::SettingsUpdate {
                        timezone: args.timezone,
                        availability,
                        horizon_days: args.horizon_days,
                        slot_minutes: args.slot_minutes,
                        solve_seconds: args.solve_seconds,
                        priority_low_weight: args.priority_low_weight,
                        priority_normal_weight: args.priority_normal_weight,
                        priority_high_weight: args.priority_high_weight,
                        cognitive_enabled: args.cognitive_enabled,
                        low_window_start: args.low_window_start,
                        low_window_end: args.low_window_end,
                        low_outside_penalty: args.low_outside_penalty,
                        medium_window_start: args.medium_window_start,
                        medium_window_end: args.medium_window_end,
                        medium_outside_penalty: args.medium_outside_penalty,
                        high_window_start: args.high_window_start,
                        high_window_end: args.high_window_end,
                        high_outside_penalty: args.high_outside_penalty,
                        high_streak_limit: args.high_streak_limit,
                        recovery_minutes: args.recovery_minutes,
                        excess_high_penalty: args.excess_high_penalty,
                    },
                )
                .map_err(planner_error)?;
                Ok(planner::settings_json(conn).map_err(planner_error)?)
            }
        },
        PlannerCommand::Optimize(args) => Ok(json!(
            planner::optimize(conn, args.horizon_days).map_err(planner_error)?
        )),
        PlannerCommand::Proposals { action } => match action {
            PlannerProposalCommand::List => {
                Ok(json!(planner::list_proposals(conn).map_err(planner_error)?))
            }
            PlannerProposalCommand::Get { id } => {
                Ok(json!(planner::proposal(conn, &id).map_err(planner_error)?))
            }
            PlannerProposalCommand::Apply { id } => Ok(json!(
                planner::apply_proposal(conn, &id).map_err(planner_error)?
            )),
        },
    }
}

fn task_priority_from_arg(value: TaskPriorityArg) -> models::TaskPriority {
    match value {
        TaskPriorityArg::Low => models::TaskPriority::Low,
        TaskPriorityArg::Normal => models::TaskPriority::Normal,
        TaskPriorityArg::High => models::TaskPriority::High,
    }
}

fn cognitive_load_from_arg(value: CognitiveLoadArg) -> models::CognitiveLoad {
    match value {
        CognitiveLoadArg::Low => models::CognitiveLoad::Low,
        CognitiveLoadArg::Medium => models::CognitiveLoad::Medium,
        CognitiveLoadArg::High => models::CognitiveLoad::High,
    }
}

fn deadline_kind_from_arg(value: DeadlineKindArg) -> models::DeadlineKind {
    match value {
        DeadlineKindArg::None => models::DeadlineKind::None,
        DeadlineKindArg::Hard => models::DeadlineKind::Hard,
        DeadlineKindArg::Soft => models::DeadlineKind::Soft,
    }
}

fn planner_error(error: planner::PlannerError) -> CliError {
    match error {
        planner::PlannerError::NotFound { resource, id } => CliError::not_found(resource, &id),
        planner::PlannerError::Validation(message) => CliError::validation(message),
        planner::PlannerError::Conflict(message) => CliError::conflict(message),
        planner::PlannerError::Internal(message) => CliError::internal(message),
    }
}
