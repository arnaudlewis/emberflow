spec emberflow-client-contract v1.1.0
title "EmberFlow client contract"

description
  Defines how any EmberFlow client bootstraps, reads canonical state through
  resources, performs durable mutations through tools, and displays tracked
  state transparency without turning projected files into the source of truth.

motivation
  EmberFlow now provides a canonical local runtime store, projected filesystem
  views, a resources-only read surface, and a write-tool MCP transport. Every
  consumer — whether it is Runestone or a future adapter — needs one stable
  client contract so the runtime is resumed, mutated, and displayed
  consistently without public low-level read methods.

behavior client-initializes-before-tracked-work [happy_path]
  "When a client starts tracked work, it initializes EmberFlow against the active workspace before reading or mutating tracked state"

  given
    A client has entered a workspace that should use EmberFlow

  when bootstrap-tracked-session

  then returns bootstrap
    assert initialize == "called"
    assert workspaceResolution is_present
    assert trackedWork == "not-started-before-initialize"

behavior client-reads-self-describing-emberflow-bootstrap [happy_path]
  "When a client initializes EmberFlow, it can learn EmberFlow's canonical role and resources-only usage sequence from the bootstrap response itself"

  given
    A client has entered a workspace that should use EmberFlow

  when bootstrap-tracked-session

  then returns bootstrap
    assert sourceOfTruth == "emberflow-canonical-state"
    assert projectedFiles == "derived-only"
    assert preferredClientSequence == ["initialize", "list_resources", "read_resource", "mutate_via_emberflow_mcp"]
    assert knowledgeViews == ["workspace-overview", "track-record", "track-resume", "track-transparency", "track-context", "track-brief", "track-plan", "track-runtime", "track-events", "task-visibility", "task-events", "client-contract"]
    assert resources == ["emberflow://workspace/overview", "emberflow://tracks/{trackId}/record", "emberflow://tracks/{trackId}/resume", "emberflow://tracks/{trackId}/transparency", "emberflow://tracks/{trackId}/context", "emberflow://tracks/{trackId}/brief", "emberflow://tracks/{trackId}/plan", "emberflow://tracks/{trackId}/runtime", "emberflow://tracks/{trackId}/events", "emberflow://tasks/{taskId}/visibility", "emberflow://tasks/{taskId}/events", "emberflow://protocol/client-contract"]
    assert capabilities == ["emberflow-track-create", "emberflow-track-metadata-upsert", "emberflow-track-brief-replace", "emberflow-track-plan-replace", "emberflow-track-archive", "emberflow-track-delete", "emberflow-task-create", "emberflow-event-record"]

behavior missing-workspace-root-fails-explicitly [error_case]
  "When a client cannot resolve a valid EmberFlow workspace, it fails explicitly instead of silently proceeding without EmberFlow"

  given
    A client has been asked to start tracked work
    The active workspace does not resolve to a valid EmberFlow root

  when bootstrap-tracked-session

  then returns failure
    assert status == "emberflow-unavailable"
    assert reason contains "workspace"
    assert silentFallback == "none"

behavior canonical-state-remains-authoritative [happy_path]
  "When a client consumes EmberFlow state, it treats the canonical EmberFlow record as authoritative and projected files as derived views only"

  given
    EmberFlow provides canonical tracked state and projected filesystem views

  when load-tracked-context

  then returns context
    assert sourceOfTruth == "emberflow-canonical-state"
    assert projectedFiles == "derived-only"
    assert projectedFilesAuthority == "none"

behavior client-loads-minimal-context-before-resume [happy_path]
  "When a client resumes tracked work, it loads the minimal EmberFlow resources needed to continue coherently"

  given
    EmberFlow has tracked state for the active work

  when resume-tracked-session

  then returns context
    assert track is_present
    assert trackTransparency is_present
    assert next is_present
    assert runtimeContext is_present

behavior client-blocks-planned-work-without-resolved-track [error_case]
  "When a client enters planned work without a resolved durable track, it blocks implementation until a track has been resolved or created"

  given
    A client has initialized EmberFlow successfully
    The client is about to begin planned work
    No durable track is currently attached to the work

  when start-planned-work

  then returns readiness
    assert status == "track-required"
    assert reason contains "track"
    assert implementation == "blocked"

  then side_effect
    assert Planned work does not continue until the client resolves or creates a durable EmberFlow track

behavior root-declares-durable-track-transitions [happy_path]
  "When tracked work crosses a durable semantic boundary, the client states that the root orchestrator layer declares the transition instead of inferring it from worker chatter"

  given
    A client is reading the EmberFlow root instruction contract for tracked work

  when load-root-contract

  then returns transitionContract
    assert rootLayer == "root-orchestrator"
    assert durableTransitionDeclaration == "present"
    assert workerOwnership == "none"

  then side_effect
    assert Semantic track transitions are declared at the root layer instead of being inferred from worker chatter

behavior advisory-updates-do-not-write-canonical-track-state [edge_case]
  "When tracked execution emits progress, blocker, or handoff messages, the client keeps those updates advisory until EmberFlow records a durable transition"

  given
    Tracked execution is underway
    A client is reading the EmberFlow root instruction contract for tracked work

  when emit-advisory-update

  then returns advisoryUpdate
    assert durable == false
    assert canonicalWrites == "none"
    assert rootLayer == "root-orchestrator"

  then side_effect
    assert Worker messages do not directly write canonical EmberFlow track metadata or plan state

behavior client-may-list-readable-resources-after-bootstrap [happy_path]
  "When a client finishes bootstrapping EmberFlow, it may discover EmberFlow's stable read-only resources through standard MCP discovery before choosing which high-level context view to load"

  given
    A client has initialized EmberFlow successfully

  when list-resources

  then returns resourceCatalog
    assert resources is_present
    assert resources contains "emberflow://workspace/overview"
    assert resources contains "emberflow://tracks/{trackId}/record"
    assert resources contains "emberflow://tracks/{trackId}/resume"
    assert resources contains "emberflow://tracks/{trackId}/transparency"
    assert resources contains "emberflow://tracks/{trackId}/context"
    assert resources contains "emberflow://tracks/{trackId}/brief"
    assert resources contains "emberflow://tracks/{trackId}/plan"
    assert resources contains "emberflow://tracks/{trackId}/runtime"
    assert resources contains "emberflow://tracks/{trackId}/events"
    assert resources contains "emberflow://tasks/{taskId}/visibility"
    assert resources contains "emberflow://tasks/{taskId}/events"
    assert resources contains "emberflow://protocol/client-contract"

behavior client-may-read-dynamic-knowledge-views-before-resume [happy_path]
  "When a client wants a high-level summary before resuming tracked work, it reads EmberFlow's dynamic knowledge resources instead of reconstructing the context manually or calling public read methods"

  given
    EmberFlow has tracked state for the active work

  when resume-tracked-session

  then returns context
    assert workspaceOverview is_present
    assert trackResume is_present
    assert trackTransparency is_present
    assert trackContext is_present
    assert manualReconstruction == "not-required"
    assert preferredReadPath == "resources-only"
    assert publicReadMethods == "none"

behavior client-may-read-display-ready-transparency-resource [happy_path]
  "When a client needs to display the current tracked state after bootstrap or mutation, it may read EmberFlow's canonical transparency resource instead of reconstructing the display block manually"

  given
    EmberFlow has tracked state for the active work

  when display-transparency

  then returns transparency
    assert resource == "emberflow://tracks/{trackId}/transparency"
    assert trackStatus is_present
    assert taskStatus is_present
    assert phase is_present
    assert next is_present

behavior durable-mutations-use-emberflow-surface [happy_path]
  "When a client performs a durable tracked mutation, it uses the EmberFlow MCP surface instead of mutating projected files directly"

  given
    A client needs to update durable tracked state

  when mutate-tracked-state

  then returns mutation
    assert transport == "emberflow-mcp"
    assert directProjectionWrite == "none"
    assert durableMutation == "through-emberflow-only"

behavior bootstrap-displays-required-transparency-fields [happy_path]
  "When a client bootstraps tracked work successfully, it displays a transparency block sourced from EmberFlow with the required canonical fields"

  given
    EmberFlow bootstrap has succeeded
    The active tracked context is available

  when display-bootstrap-transparency

  then returns transparencyBlock
    assert source == "EmberFlow"
    assert track is_present
    assert trackStatus is_present
    assert taskStatus is_present
    assert phase is_present
    assert next is_present

behavior transparency-may-include-enriched-task-visibility [happy_path]
  "When EmberFlow provides canonical task visibility fields, a client may display task id, executor, execution, intent_summary, and updated_at alongside the required transparency fields"

  given
    EmberFlow has required transparency state for tracked work
    EmberFlow also provides task visibility enrichment for the current task

  when display-transparency

  then returns transparencyBlock
    assert source == "EmberFlow"
    assert track is_present
    assert trackStatus is_present
    assert taskStatus is_present
    assert phase is_present
    assert next is_present
    assert taskId is_present
    assert executor is_present
    assert execution is_present
    assert intent_summary is_present
    assert updated_at is_present

behavior missing-transparency-fields-are-marked-unavailable [edge_case]
  "When EmberFlow does not provide a required or recommended transparency field, the client displays 'unavailable from EmberFlow' instead of inventing a value"

  given
    The client is displaying EmberFlow transparency
    One or more transparency fields are absent from the EmberFlow response

  when display-transparency

  then returns transparencyBlock
    assert missingFieldValue == "unavailable from EmberFlow"
    assert taskStatus == "unavailable from EmberFlow"
    assert phase == "unavailable from EmberFlow"
    assert executor == "unavailable from EmberFlow"
    assert execution == "unavailable from EmberFlow"
    assert intent_summary == "unavailable from EmberFlow"
    assert guessedValue == "none"

behavior post-mutation-transparency-reloads-canonical-state [happy_path]
  "When a client completes a durable tracked mutation, it reloads canonical EmberFlow state before displaying the updated transparency block"

  given
    A durable tracked mutation has succeeded through EmberFlow

  when display-post-mutation-transparency

  then returns transparencyBlock
    assert canonicalReload == "performed-after-mutation"
    assert source == "EmberFlow"
    assert trackStatus is_present
    assert taskStatus is_present
    assert phase is_present
    assert next is_present

behavior handoff-displays-current-emberflow-state [happy_path]
  "When a client hands tracked work to another agent or task, it displays the current EmberFlow transparency state before the handoff proceeds"

  given
    Tracked work is active
    A handoff is about to occur

  when display-handoff-transparency

  then returns handoffBlock
    assert source == "EmberFlow"
    assert track is_present
    assert trackStatus is_present
    assert taskStatus is_present
    assert phase is_present
    assert next is_present

behavior emberflow-unavailable-never-produces-guessed-canonical-state [error_case]
  "When EmberFlow becomes unavailable during tracked work, the client reports the failure explicitly and never presents guessed state as canonical"

  given
    A client is using EmberFlow for tracked work
    EmberFlow is unavailable for the requested operation

  when display-unavailable-state

  then returns failureBlock
    assert source == "unavailable"
    assert status == "emberflow-unavailable"
    assert next == "unavailable from EmberFlow"
    assert guessedCanonicalState == "none"

behavior contract-applies-uniformly-to-all-clients [edge_case]
  "When different clients consume EmberFlow, they follow the same bootstrap, mutation, and transparency contract without a privileged client path"

  given
    More than one EmberFlow client exists

  when compare-client-contract

  then returns contractComparison
    assert runestonePath == "standard-client"
    assert otherClients == "same-contract"
    assert privilegedClientPath == "none"

depends on emberflow-mcp-surface >= 1.0.0
depends on emberflow-mcp-stdio-transport >= 1.0.0
depends on emberflow-project-layout >= 1.0.0
