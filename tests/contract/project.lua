-- Real-core project switching checks, run by scripts/rust-port/startup-check.sh.
local core = require "core"
local command = require "core.command"
local common = require "core.common"
local M = {}

function M.init()
  core.init()
  local next_project = os.getenv("PROJECT_NEXT")
  if RESTARTED then
    assert(core.root_project().path == next_project, "restart did not open the selected project")
    io.stderr:write("PROJECT restored\n")
    return
  end

  local caller, action = os.getenv("PROJECT_CASE"):match("^(%w+)%-(%w+)$")
  local project = core.root_project()
  local recents = common.serialize(core.recent_projects)
  local doc = core.open_doc()
  local view = core.root_view:open_doc(doc)
  if action ~= "clean" then doc:insert(1, 1, "unsaved text\n") end
  local text = table.concat(doc.lines)
  local prompts = 0
  local show = core.nag_view.show
  function core.nag_view:show(...)
    prompts = prompts + 1
    return show(self, ...)
  end

  if caller == "command" then
    assert(command.perform("core:change-project-folder-commandview"))
    core.command_view:set_text(next_project)
    core.command_view:submit()
  elseif caller == "restart" then
    assert(command.perform("core:restart"))
  else
    core.open_project(next_project)
  end

  if action ~= "clean" then
    assert(core.nag_view.visible, "unsaved changes prompt missing")
    assert(core.root_project() == project, "project changed before confirmation")
    assert(core.get_views_referencing_doc(doc)[1] == view, "view closed before confirmation")
    assert(common.serialize(core.recent_projects) == recents, "recents changed before confirmation")
    assert(command.perform(action == "cancel" and "dialog:select-no" or "dialog:select-yes"))
  end
  assert(prompts == (action == "clean" and 0 or 1), "project switch prompted more than once")

  if action == "cancel" then
    core.redraw = true
    core.step()
    assert(core.root_project() == project, "cancel changed the project")
    assert(common.serialize(core.recent_projects) == recents, "cancel changed recent projects")
    assert(core.docs[1] == doc, "cancel discarded the unsaved document")
    assert(core.get_views_referencing_doc(doc)[1] == view, "cancel closed the document view")
    assert(doc:is_dirty() and table.concat(doc.lines) == text, "cancel changed unsaved text")
    assert(not core.restart_request and not core.quit_request, "cancel requested restart/quit")
    io.stderr:write("PROJECT cancelled\n")
  else
    assert(core.restart_request, "confirmed/clean project switch did not restart")
    assert(not core.nag_view.visible, "project switch left a second confirmation open")
  end
end

function M.run() M.restart_request = core.restart_request end
function M.on_error(err) io.stderr:write(tostring(err), "\n") end
return M
