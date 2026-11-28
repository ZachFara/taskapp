/// Calls `$m! { <every command> }`. The app passes its own wrapper-generating macro.
#[macro_export]
macro_rules! with_commands {
    ($m:ident) => {
        $crate::with_commands!($m, ChangeSource::Ui);
    };
    ($m:ident, $source:expr) => {
        $m! {
            issues_create(s, input: NewIssue) -> Issue { issues::create(&s.db, input, $source).await }
            issues_update(s, id: IssueId, patch: IssuePatch) -> Issue {
                issues::update(&s.db, id, patch, $source).await
            }
            issues_bulk_update(s, ids: Vec<IssueId>, patch: BulkPatch) -> () {
                bulk::update(&s.db, &ids, patch, $source).await
            }
            issues_mark_duplicate(s, id: IssueId, of: IssueId) -> Issue {
                relations::mark_duplicate(&s.db, id, of, $source).await
            }
            issues_duplicate(s, ids: Vec<IssueId>) -> Vec<Issue> {
                bulk::duplicate(&s.db, &ids, $source).await
            }
            issues_delete(s, ids: Vec<IssueId>) -> u32 { delete::remove(&s.db, &ids, $source).await }
            issues_empty_archive(s) -> u32 { delete::empty_archive(&s.db, $source).await }
            issues_get(s, id: IssueId) -> Issue { issues::get(&s.db, id).await }
            issues_get_by_key(s, key: String) -> Issue { issues::get_by_key(&s.db, &key).await }
            issues_archive(s, id: IssueId) -> () { issues::archive(&s.db, id, $source).await }
            issues_restore(s, id: IssueId) -> () { issues::restore(&s.db, id, $source).await }
            issues_children(s, id: IssueId) -> Vec<Issue> { issues::children(&s.db, id).await }
            issues_parse_quickadd(s, input: String) -> ParsedQuickAdd {
                quickadd::parse_for(&s.db, &input, today(s)?, &s.tz).await
            }
            issues_create_from_quickadd(s, input: String) -> Issue {
                quickadd::create_from(&s.db, &input, today(s)?, &s.tz, $source).await
            }

            inbox_list(s) -> Vec<Issue> { inbox::list(&s.db, today(s)?).await }
            inbox_process(s, id: IssueId, choice: InboxChoice) -> Issue {
                inbox::process(&s.db, id, choice, today(s)?, $source).await
            }

            workspace_status(s) -> WorkspaceStatus { workspace::status(&s.db).await }
            workspace_setup(s, name: String, key: String, timezone: Option<String>) -> WorkspaceStatus {
                workspace::setup(&s.db, &name, &key, timezone.as_deref()).await
            }

            statuses_list(s) -> Vec<Status> { statuses::list(&s.db).await }
            statuses_create(s, name: String, category: StatusCategory, color: String, icon: Option<String>) -> Status {
                statuses::create(&s.db, &name, category, &color, icon, $source).await
            }
            statuses_update(s, id: StatusId, name: Option<String>, color: Option<String>, position: Option<i32>, icon: Option<String>)
                -> Status { statuses::update(&s.db, id, name, position, taskapp_core::palette::Look { icon, color }, $source).await }
            statuses_set_flow(s, id: StatusId, definition: Option<String>, flow_kind: Option<FlowKind>, nudge_days: Option<i32>)
                -> Status { statuses::set_flow(&s.db, id, definition, flow_kind, nudge_days, $source).await }
            statuses_reorder(s, ids_in_order: Vec<StatusId>) -> () {
                statuses::reorder(&s.db, &ids_in_order, $source).await
            }
            statuses_delete(s, id: StatusId, move_issues_to: StatusId) -> () {
                statuses::delete(&s.db, id, move_issues_to).await
            }

            properties_list_defs(s) -> Vec<PropertyDef> { properties::list_defs(&s.db).await }
            properties_create_def(s, def: PropertyDef) -> PropertyDef {
                properties::create_def(&s.db, def, $source).await
            }
            properties_rename_def(s, key: String, new_name: String) -> PropertyDef {
                properties::rename_def(&s.db, &key, &new_name, $source).await
            }
            properties_change_kind(s, key: String, kind: PropertyKind, options: Vec<SelectOption>) -> PropertyDef {
                properties::change_kind(&s.db, &key, kind, options, $source).await
            }
            properties_reorder(s, keys: Vec<String>) -> () { properties::reorder(&s.db, &keys).await }
            properties_set_scope(s, key: String, project: Option<ProjectId>) -> PropertyDef {
                properties::set_scope(&s.db, &key, project, $source).await
            }
            properties_set_applies_to(s, key: String, applies_to: PropertyAppliesTo) -> PropertyDef {
                properties::set_applies_to(&s.db, &key, applies_to, $source).await
            }
            properties_delete_def(s, key: String) -> () { properties::delete_def(&s.db, &key, $source).await }
            properties_set_value(s, issue: IssueId, key: String, value: Option<Value>) -> Issue {
                properties::set_value(&s.db, issue, &key, value, $source).await
            }

            project_statuses_list(s) -> Vec<ProjectStatus> { project_statuses::list(&s.db).await }
            project_statuses_create(s, name: String, category: ProjectStatusCategory, color: String, icon: Option<String>)
                -> ProjectStatus {
                project_statuses::create(&s.db, &name, category, &color, icon, $source).await
            }
            project_statuses_update(s, id: ProjectStatusId, name: Option<String>, color: Option<String>, icon: Option<String>)
                -> ProjectStatus {
                project_statuses::update(&s.db, id, name, taskapp_core::palette::Look { icon, color }, $source).await
            }
            project_statuses_reorder(s, ids_in_order: Vec<ProjectStatusId>) -> () {
                project_statuses::reorder(&s.db, &ids_in_order, $source).await
            }
            project_statuses_delete(s, id: ProjectStatusId, move_projects_to: ProjectStatusId) -> () {
                project_statuses::delete(&s.db, id, move_projects_to, $source).await
            }

            projects_list(s) -> Vec<Project> { projects::list(&s.db).await }
            projects_create(s, slug: String, name: String, icon: Option<String>, color: Option<String>) -> Project {
                projects::create(&s.db, &slug, &name, icon, color, $source).await
            }
            projects_update(s, project: Project) -> Project { projects::update(&s.db, project, $source).await }
            projects_delete(s, id: ProjectId) -> () { projects::delete(&s.db, id, $source).await }
            next_list(s) -> Vec<NextAction> { next_actions::list(&s.db).await }
            projects_set_next(s, project: ProjectId, issue: Option<IssueId>) -> () {
                next_actions::set(&s.db, project, issue, $source).await
            }
            projects_progress(s, id: ProjectId) -> (u32, u32) { projects::progress(&s.db, id).await }
            projects_list_milestones(s, project: ProjectId) -> Vec<Milestone> {
                projects::list_milestones(&s.db, project).await
            }
            projects_create_milestone(s, project: ProjectId, name: String, target_date: Option<NaiveDate>)
                -> Milestone {
                projects::create_milestone(&s.db, project, &name, target_date, $source).await
            }
            projects_update_milestone(s, milestone: Milestone) -> Milestone {
                projects::update_milestone(&s.db, milestone, $source).await
            }
            projects_delete_milestone(s, id: MilestoneId) -> () {
                projects::delete_milestone(&s.db, id, $source).await
            }

            cycles_list(s) -> Vec<Cycle> { cycles::list(&s.db).await }
            cycles_create(s, name: Option<String>, start_date: NaiveDate, end_date: NaiveDate, icon: Option<String>, color: Option<String>) -> Cycle {
                cycles::create(&s.db, name, start_date, end_date, taskapp_core::palette::Look { icon, color }, $source).await
            }
            cycles_update(s, cycle: Cycle) -> Cycle { cycles::update(&s.db, cycle, $source).await }
            cycles_delete(s, id: CycleId) -> () { cycles::delete(&s.db, id, $source).await }

            events_list(s, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Vec<Event> {
                events::list(&s.db, from, to).await
            }
            events_get(s, id: EventId) -> Event { events::get(&s.db, id).await }
            events_create(s, input: EventInput) -> Event { events::create(&s.db, input, $source).await }
            events_update(s, id: EventId, input: EventInput) -> Event {
                events::update(&s.db, id, input, $source).await
            }
            events_delete(s, id: EventId) -> () { events::delete(&s.db, id, $source).await }
            events_link(s, event: EventId, issue: IssueId) -> () {
                events::link(&s.db, event, issue, $source).await
            }
            events_unlink(s, event: EventId, issue: IssueId) -> () {
                events::unlink(&s.db, event, issue, $source).await
            }

            decisions_list(s, status: Option<DecisionStatus>) -> Vec<Decision> {
                decisions::list(&s.db, status).await
            }
            decisions_get(s, id: DecisionId) -> Decision { decisions::get(&s.db, id).await }
            decisions_create(s, input: DecisionInput) -> Decision {
                decisions::create(&s.db, input, $source).await
            }
            decisions_answer(s, id: DecisionId, option: Option<String>, text: Option<String>, note: String) -> Decision {
                decisions::answer(&s.db, id, option, text, note, $source).await
            }
            decisions_withdraw(s, id: DecisionId) -> Decision { decisions::withdraw(&s.db, id, $source).await }
            decisions_link(s, decision: DecisionId, issue: IssueId) -> () {
                decisions::link(&s.db, decision, issue, $source).await
            }
            decisions_unlink(s, decision: DecisionId, issue: IssueId) -> () {
                decisions::unlink(&s.db, decision, issue, $source).await
            }

            links_list(s, issue: IssueId) -> Vec<Link> { links::list(&s.db, issue).await }
            links_add(s, issue: IssueId, url: String, title: Option<String>, kind: Option<LinkKind>) -> Link {
                links::add(&s.db, issue, &url, title, kind, $source).await
            }
            links_remove(s, id: LinkId) -> () { links::remove(&s.db, id, $source).await }

            labels_list(s) -> Vec<Label> { labels::list(&s.db).await }
            labels_create(s, name: String, color: String, icon: Option<String>) -> Label {
                labels::create(&s.db, &name, &color, icon, $source).await
            }
            labels_update(s, label: Label) -> Label { labels::update(&s.db, label, $source).await }
            labels_delete(s, id: LabelId) -> () { labels::delete(&s.db, id, $source).await }
            labels_add_label(s, issue: IssueId, label: LabelId) -> () {
                labels::add_label(&s.db, issue, label, $source).await
            }
            labels_remove_label(s, issue: IssueId, label: LabelId) -> () {
                labels::remove_label(&s.db, issue, label, $source).await
            }

            templates_list(s) -> Vec<IssueTemplate> { templates::list(&s.db).await }
            templates_create(s, input: TemplateInput) -> IssueTemplate {
                templates::create(&s.db, input, $source).await
            }
            templates_update(s, id: TemplateId, input: TemplateInput) -> IssueTemplate {
                templates::update(&s.db, id, input, $source).await
            }
            templates_delete(s, id: TemplateId) -> () { templates::delete(&s.db, id, $source).await }
            templates_apply(s, id: TemplateId, overrides: NewIssue) -> AppliedTemplate {
                templates::apply(&s.db, id, overrides, $source).await
            }

            comments_list(s, issue: IssueId) -> Vec<Comment> { comments::list(&s.db, issue).await }
            comments_add_comment(s, issue: IssueId, body: String) -> Comment {
                comments::add_comment(&s.db, issue, &body, None, $source).await
            }
            comments_update(s, id: CommentId, body: String) -> Comment {
                comments::update(&s.db, id, &body, $source).await
            }
            comments_delete(s, id: CommentId) -> () { comments::delete(&s.db, id, $source).await }

            relations_add(s, rel: Relation) -> () { relations::add(&s.db, rel, $source).await }
            relations_remove(s, rel: Relation) -> () { relations::remove(&s.db, rel, $source).await }
            relations_for_issue(s, id: IssueId) -> Vec<Relation> { relations::for_issue(&s.db, id).await }
            relation_kinds_list(s) -> Vec<RelationKindDef> { relation_kinds::list(&s.db).await }
            relation_kinds_create(s, name: String, forward: String, reverse: String, symmetric: bool) -> RelationKindDef {
                relation_kinds::create(&s.db, &name, &forward, &reverse, symmetric, $source).await
            }
            relation_kinds_update(s, kind: RelationKindDef) -> RelationKindDef {
                relation_kinds::update(&s.db, kind, $source).await
            }
            relation_kinds_delete(s, id: RelationKind) -> () { relation_kinds::delete(&s.db, &id, $source).await }

            // They take the definition, not a saved view: a project page shows a view that is never saved.
            schedule_bars(s, def: ViewDef) -> Vec<Bar> {
                Ok(views::query(&s.db, &def).await?.iter().filter_map(schedule::bar_for).collect())
            }
            schedule_critical_path(s, def: ViewDef) -> Vec<IssueId> {
                Ok(schedule::critical_path(&views::query(&s.db, &def).await?, &relations::all_blocking(&s.db).await?))
            }
            schedule_shift_dependents(s, moved: Reschedule) -> Vec<Reschedule> {
                let all = views::query(&s.db, &all_issues_def()).await?;
                Ok(schedule::shift_dependents(&all, &relations::all_blocking(&s.db).await?, &moved))
            }

            views_query(s, def: ViewDef) -> Vec<Issue> { views::query(&s.db, &def).await }
            views_list(s) -> Vec<SavedView> { views::list(&s.db).await }
            views_query_projects(s, def: ViewDef) -> Vec<ProjectRow> { views::projects_query(&s.db, &def).await }
            views_save(s, id: Option<ViewId>, name: String, def: ViewDef, icon: Option<String>, color: Option<String>) -> SavedView {
                views::save(&s.db, id, &name, def, taskapp_core::palette::Look { icon, color }, $source).await
            }
            // A new view lists projects; an existing one keeps what it lists.
            views_save_project(s, id: Option<ViewId>, name: String, def: ViewDef, icon: Option<String>, color: Option<String>) -> SavedView {
                views::save_as(&s.db, id, (&name, ViewEntity::Project), def, taskapp_core::palette::Look { icon, color }, $source).await
            }
            // The tab a view was last shown in; no undo step (picking a tab is not an edit).
            views_set_layout(s, id: ViewId, layout: taskapp_core::Layout) -> () { views::set_layout(&s.db, id, layout).await }
            views_delete(s, id: ViewId) -> () { views::delete(&s.db, id).await }
            views_reorder(s, ids: Vec<ViewId>) -> () { views::reorder(&s.db, &ids).await }
            attachments_add(s, bytes: Vec<u8>, mime: String) -> Attachment {
                attachments::add(&s.db, &bytes, &mime).await
            }
            attachments_get(s, id: AttachmentId) -> AttachmentData { attachments::get(&s.db, id).await }

            folders_list(s) -> Vec<ViewFolder> { view_folders::list(&s.db).await }
            folders_items(s) -> Vec<Placement> { view_folders::placements(&s.db).await }
            folders_create(s, section: FolderSection, parent_id: Option<ViewFolderId>, name: String) -> ViewFolder {
                view_folders::create(&s.db, section, parent_id, &name, $source).await
            }
            folders_rename(s, id: ViewFolderId, name: String) -> ViewFolder {
                view_folders::rename(&s.db, id, &name, $source).await
            }
            folders_delete(s, id: ViewFolderId) -> () { view_folders::delete(&s.db, id, $source).await }
            folders_place(s, id: ViewFolderId, parent_id: Option<ViewFolderId>, index: Option<u32>) -> () {
                view_folders::place_folder(&s.db, id, parent_id, index, $source).await
            }
            folders_place_item(s, section: FolderSection, id: i64, folder_id: Option<ViewFolderId>, index: Option<u32>) -> () {
                view_folders::place_item(&s.db, section, id, folder_id, index, $source).await
            }
            views_stats(s, def: ViewDef) -> Vec<IssueStats> { views::stats(&s.db, &def).await }
            views_create_from_template(s, template_id: ViewTemplateId, name: Option<String>) -> SavedView {
                view_templates::create_view(&s.db, template_id, name.as_deref(), $source).await
            }
            views_save_as_template(s, view_id: ViewId, name: Option<String>) -> ViewTemplate {
                view_templates::from_view(&s.db, view_id, name.as_deref(), $source).await
            }
            views_reset_to_template(s, view_id: ViewId) -> SavedView {
                view_templates::reset_view(&s.db, view_id, $source).await
            }
            view_templates_list(s) -> Vec<ViewTemplate> { view_templates::list(&s.db).await }
            view_templates_create(s, name: String, def: ViewDef, icon: Option<String>, color: Option<String>) -> ViewTemplate {
                view_templates::create(&s.db, &name, def, icon, color, $source).await
            }
            view_templates_update(s, template: ViewTemplate) -> ViewTemplate {
                view_templates::update(&s.db, template, $source).await
            }
            view_templates_delete(s, id: ViewTemplateId) -> () { view_templates::delete(&s.db, id, $source).await }

            import_csv_plan(s, csv: String, mapping: Option<Vec<(String, String)>>) -> ImportPlan {
                let mapping = mapping.map(|m| m.into_iter().collect());
                import::csv::plan(&s.db, &csv, mapping.as_ref()).await
            }
            import_apply(s, plan: ImportPlan) -> Applied { import::apply(&s.db, &plan).await }

            insights_progress(s, scope: ProgressScope, measure: Measure, from: Option<NaiveDate>, to: Option<NaiveDate>)
                -> ProgressSeries { insights::progress(&s.db, scope, measure, from, to).await }
            insights_velocity(s, scope: ProgressScope, measure: Measure, periods: u32) -> Vec<VelocityPeriod> {
                insights::velocity(&s.db, scope, measure, periods).await
            }

            forecast_for(s, query: ForecastQuery) -> Forecast { forecast::forecast_for(&s.db, query).await }
            metrics_for(s, query: MetricsQuery) -> Metrics { metrics::metrics_for(&s.db, query).await }

            stale_list(s) -> Vec<StaleIssue> { stale::list(&s.db).await }

            focus_start(s, issue: IssueId, deep: Option<bool>) -> FocusSession {
                focus::start(&s.db, issue, deep, $source).await
            }
            focus_stop(s) -> Option<FocusSession> { focus::stop(&s.db, $source).await }
            focus_running(s) -> Option<FocusSession> { focus::running(&s.db).await }
            focus_log(s, issue: Option<IssueId>, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Vec<FocusSession> {
                focus::log(&s.db, issue, from, to).await
            }

            search_search(s, text: String, limit: u32) -> Vec<Issue> { search::search(&s.db, &text, limit).await }
            search_similar(s, text: String, project_id: Option<ProjectId>, limit: u32) -> Vec<Issue> {
                search::similar(&s.db, &text, project_id, limit).await
            }

            pins_pin(s, kind: PinKind, target: String, title: Option<String>) -> Pin {
                pins::pin(&s.db, kind, &target, title).await
            }
            pins_set(s, kind: PinKind, targets: Vec<String>, pinned: bool) -> () {
                pins::set(&s.db, kind, &targets, pinned).await
            }
            pins_unpin(s, id: PinId) -> () { pins::unpin(&s.db, id).await }
            pins_reorder(s, ids_in_order: Vec<PinId>) -> () { pins::reorder(&s.db, &ids_in_order).await }
            pins_list_resolved(s) -> Vec<ResolvedPin> { pins::list_resolved(&s.db).await }

            blocks_create(s, issue: IssueId, start: DateTime<Utc>, end: DateTime<Utc>) -> WorkBlock {
                blocks::create_block(&s.db, issue, start, end).await
            }
            blocks_move(s, id: WorkBlockId, start: DateTime<Utc>, end: DateTime<Utc>) -> WorkBlock {
                blocks::move_block(&s.db, id, start, end).await
            }
            blocks_delete(s, id: WorkBlockId) -> () { blocks::delete_block(&s.db, id).await }
            blocks_between(s, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<WorkBlock> {
                blocks::blocks_between(&s.db, from, to).await
            }

            history_list_for(s, entity: String, entity_id: String) -> Vec<Change> {
                history::list_for(&s.db, &entity, &entity_id).await
            }
            history_undo(s) -> UndoStatus { undo::undo(&s.db).await }
            history_redo(s) -> UndoStatus { undo::redo(&s.db).await }
            history_undo_status(s) -> UndoStatus { undo::undo_status(&s.db).await }
            settings_get(s, key: String) -> Option<Value> { settings::get(&s.db, &key).await }
            settings_set(s, key: String, value: Value) -> () { settings::set_from_user(&s.db, &key, &value).await }
            backup_snapshot(s, dir: String, reason: String) -> String {
                Ok(backup::snapshot(&s.db, dir.as_ref(), &reason).await?.to_string_lossy().into_owned())
            }

            backup_list(_s, dir: String) -> Vec<String> {
                Ok(backup::list(dir.as_ref())?.into_iter().map(|p| p.to_string_lossy().into_owned()).collect())
            }
            sync_status(s) -> SyncStatus {
                Ok(settings::get(&s.db, "sync_status").await?.unwrap_or(SyncStatus {
                    reminders: SyncState::Off,
                    gcal: SyncState::Off,
                    export: SyncState::Off,
                    backup: SyncState::Off,
                }))
            }
        }
    };
}
