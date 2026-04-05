spec emberflow-project-layout v1.0.0
title "EmberFlow project layout"

description
  Defines how EmberFlow resolves the shared project root, canonical SQLite
  state, and optional projected filesystem state for a project.

motivation
  EmberFlow needs one shared canonical database per project, regardless of
  whether agents run in a git worktree or a plain directory, while keeping
  filesystem projections optional and deterministic.

behavior emberflow-resolves-project-root-from-git-common-dir [happy_path]
  "When EmberFlow runs inside a git worktree, it resolves the shared project root from the parent of git common dir"

  given
    A git-backed workspace is available

  when resolve-project-root
    workspace = "git-worktree"

  then returns projectLayout
    assert projectRoot is_present
    assert emberflowRoot contains ".emberflow"
    assert dbPath contains ".emberflow/emberflow.db"
    assert mode == "canonical"

behavior emberflow-falls-back-to-local-root-without-git [edge_case]
  "When EmberFlow runs outside git, it resolves the project root to the current directory instead of failing"

  given
    A non-git workspace is available

  when resolve-project-root
    workspace = "plain-directory"

  then returns projectLayout
    assert projectRoot is_present
    assert emberflowRoot contains ".emberflow"
    assert dbPath contains ".emberflow/emberflow.db"
    assert mode == "canonical"

behavior emberflow-honors-config-root-override [happy_path]
  "When emberflow.config.json provides a root override, EmberFlow uses that root for the shared .emberflow state"

  given
    A workspace includes emberflow.config.json

  when resolve-project-root
    mode = "projected"
    root = "../shared-project"

  then returns projectLayout
    assert projectRoot contains "shared-project"
    assert mode == "projected"
    assert emberflowRoot contains ".emberflow"
    assert dbPath contains ".emberflow/emberflow.db"

behavior emberflow-runtime-new-keeps-legacy-db-path [happy_path]
  "When a legacy caller still uses EmberFlowRuntime::new with an explicit db_path, EmberFlow preserves that exact db_path instead of treating the input as a workspace root"

  given
    A legacy runtime caller already has a concrete database path

  when open-compatibility-db-path
    dbPath = "./legacy.db"

  then returns projectLayout
    assert dbPath == "./legacy.db"
    assert mode == "canonical"

behavior emberflow-surface-new-keeps-legacy-db-path [happy_path]
  "When a legacy caller still uses EmberFlowSurface::new with an explicit db_path, EmberFlow preserves that exact db_path instead of treating the input as a workspace root"

  given
    A legacy surface caller already has a concrete database path

  when open-compatibility-db-path
    dbPath = "./legacy.db"

  then returns projectLayout
    assert dbPath == "./legacy.db"
    assert mode == "canonical"

behavior emberflow-projects-filesystem-artifacts-under-emberflow [happy_path]
  "When EmberFlow materializes filesystem projections, it places them under .emberflow rather than root-level .context or .tracks directories"

  given
    A projected EmberFlow layout is active

  when project-filesystem-paths
    workspace = "projected"

  then returns filesystemPaths
    assert runtimeStatusPath == ".emberflow/context/status.md"
    assert trackDirectoryPrefix == ".emberflow/tracks/"
