import { CompanionArchiveClient } from './companion-archive-client.js';
import { companionEngineConnections } from './companion-connection.js';
export async function addCompanionEngine(provider) {
  if (window.ironwoodCompanionFeatureEnabled?.() === false) return false;
  if (document.querySelector('#companion-add-engine')) return false;
  if (!document.querySelector('#companion-engines-style')) {
    const style = document.createElement('link'); style.id = 'companion-engines-style';
    style.rel = 'stylesheet'; style.href = '/companion-engines.css'; document.head.append(style);
  }
  const shared = provider || companionEngineConnections()[0]; if (!shared) return false;
  const client = new CompanionArchiveClient(shared.endpoint, shared.credential);
  return new Promise(resolve => {
    const dialog = document.createElement('dialog'); dialog.id = 'companion-add-engine';
    dialog.innerHTML = `<form>
      <header data-drag><p class="eyebrow">IRONWOOD COMPANION</p><h2>Add an engine</h2><span class="drag-hint">Drag to move · Resize from the corner</span></header>
      <div class="engine-dialog-body">
        <p>Choose a folder, then select its engine executable. Keep the engine and its supporting files together under engines.</p>
        <section data-picker><h3>1. Choose an engine folder</h3>
          <div class="engine-browser">
            <nav class="engine-folder-tree" aria-label="Engine folders"><details data-tree-root open><summary>engines</summary><div data-branches></div></details></nav>
            <div class="engine-executables"><div class="engine-file-heading"><strong>2. Select an executable</strong><button type="button" data-refresh class="secondary">Refresh folder</button></div><p data-location>engines</p><ul class="engine-file-list" aria-label="Engine executables"></ul></div>
          </div>
        </section>
        <section><h3>Selected engine</h3><label>Executable<input name="executable" readonly placeholder="Choose a folder and executable above"></label>
          <label>Engine name<input name="label" maxlength="128" placeholder="Detected automatically after selection" disabled></label>
          <details class="engine-advanced"><summary>Advanced: command-line arguments</summary><label>Arguments as a JSON array<input name="args" value="[ ]" spellcheck="false"></label><p>Usually leave this as <code>[ ]</code> (no arguments). Each argument must be a separate string.</p><button type="button" data-revalidate class="secondary">Validate again</button></details>
        </section>
      </div>
      <p role="status" aria-live="polite">Reading engine folders…</p>
      <footer class="actions"><button type="submit" disabled>Save engine</button><button type="button" data-close class="secondary">Cancel</button></footer>
    </form>`;
    const form = dialog.querySelector('form'), status = dialog.querySelector('[role=status]'), submit = form.querySelector('[type=submit]');
    const picker = form.querySelector('[data-picker]'), list = form.querySelector('.engine-file-list');
    const root = form.querySelector('[data-tree-root]');
    let alive = true, timer, added = false, saving = false, requestId, revision = 0, browseRevision = 0, selectedFolder, selectedBranch = root;
    const sameConnection = () => { return window.ironwoodCompanionFeatureEnabled?.() !== false && companionEngineConnections().some(value => value.endpoint === shared.endpoint && value.credential === shared.credential); };
    const current = turn => alive && turn === revision && sameConnection();
    async function discard() {
      const previous = requestId; requestId = undefined; clearTimeout(timer); submit.disabled = true;
      if (previous) await client.request(`engine-registrations/${previous}/cancel`, {}).catch(() => {});
    }
    async function poll(id, turn) {
      try {
        const value = await client.request(`engine-registrations/${id}`);
        if (!current(turn)) return;
        if (value.status === 'validated') {
          form.elements.label.value = value.name; form.elements.label.disabled = false;
          status.textContent = 'UCI validation passed. Ready to save.'; submit.disabled = false; return;
        }
        if (['failed', 'cancelled'].includes(value.status)) { status.textContent = value.message || 'Validation cancelled.'; return; }
        timer = setTimeout(() => void poll(id, turn), 300);
      } catch (error) { if (current(turn)) status.textContent = error.message; }
    }
    async function validate() {
      if (saving) return;
      const turn = ++revision; await discard();
      if (!current(turn) || !form.elements.executable.value) return;
      form.elements.label.disabled = true; form.elements.label.value = '';
      status.textContent = 'Checking UCI compatibility…';
      try {
        const args = JSON.parse(form.elements.args.value);
        if (!Array.isArray(args) || args.some(value => typeof value !== 'string')) throw Error('Arguments must be a JSON array of strings.');
        const value = await client.request('engine-registrations', { executable: form.elements.executable.value, args });
        if (!current(turn)) { if (value.request) void client.request(`engine-registrations/${value.request}/cancel`, {}).catch(() => {}); return; }
        if (!/^[a-f0-9]{32}$/.test(value.request)) throw Error('Unexpected validation response.');
        requestId = value.request; void poll(requestId, turn);
      } catch (error) { if (current(turn)) status.textContent = error.message; }
    }
    async function browse(path, branch = root) {
      if (saving) return;
      const turn = ++browseRevision;
      ++revision; void discard();
      form.elements.executable.value = ''; form.elements.label.value = ''; form.elements.label.disabled = true;
      picker.hidden = false; list.replaceChildren(); status.textContent = 'Reading engine folders…';
      try {
        const value = await client.request(`engine-files${path ? `?path=${encodeURIComponent(path)}` : ''}`);
        if (!alive || turn !== browseRevision || !sameConnection()) return;
        selectedFolder = value.path; selectedBranch = branch;
        form.querySelector('[data-location]').textContent = value.folder || 'engines';
        for (const summary of form.querySelectorAll('.engine-folder-tree summary')) summary.removeAttribute('aria-current');
        branch.querySelector('summary').setAttribute('aria-current', 'location');
        branch.open = true;
        const branches = branch.querySelector('[data-branches]');
        if (!branch.dataset.loaded) {
          for (const entry of value.entries.filter(entry => entry.directory)) {
            const folder = document.createElement('details'), summary = document.createElement('summary'), children = document.createElement('div');
            summary.textContent = entry.name; children.dataset.branches = '';
            summary.onclick = event => {
              event.preventDefault();
              if (folder.open && selectedBranch === folder) folder.open = false;
              else void browse(entry.path, folder);
            };
            folder.append(summary, children); branches.append(folder);
          }
          branch.dataset.loaded = 'true';
        }
        for (const entry of value.entries) {
          if (entry.directory) continue;
          const item = document.createElement('li'), button = document.createElement('button'); button.type = 'button';
          button.textContent = entry.name;
          button.onclick = () => {
            for (const other of list.querySelectorAll('button')) other.removeAttribute('aria-pressed');
            button.setAttribute('aria-pressed', 'true');
            form.elements.executable.value = entry.path; void validate();
          };
          item.append(button); list.append(item);
        }
        if (!list.children.length) { const item = document.createElement('li'); item.className = 'engine-files-empty'; item.textContent = 'No executables in this folder. Select another folder in the tree.'; list.append(item); }
        status.textContent = 'Choose a folder on the left, then an executable on the right.';
      } catch (error) { if (alive && turn === browseRevision) status.textContent = error.status === 404 ? 'This Companion build does not support engine browsing. Start the updated Companion build.' : error.message; }
    }
    root.querySelector('summary').onclick = event => { event.preventDefault(); if (root.open && selectedBranch === root) root.open = false; else void browse(undefined, root); };
    form.querySelector('[data-refresh]').onclick = () => { selectedBranch.dataset.loaded = ''; selectedBranch.querySelector('[data-branches]').replaceChildren(); void browse(selectedFolder, selectedBranch); };
    form.querySelector('[data-revalidate]').onclick = () => void validate();
    form.elements.args.oninput = () => { ++revision; void discard(); form.elements.label.disabled = true; status.textContent = 'Arguments changed. Click Validate again before saving.'; };
    form.onsubmit = async event => {
      event.preventDefault(); if (!alive || !sameConnection()) { status.textContent = 'Connection changed. Close this window and try again.'; return; }
      if (!requestId || submit.disabled) return;
      const turn = revision; saving = true; submit.disabled = true;
      // Freeze the validated selection while Save commits it.
      for (const control of form.querySelectorAll('input,button')) control.disabled = true;
      try {
        await client.request(`engine-registrations/${requestId}/save`, { label: form.elements.label.value.trim() });
        if (!current(turn)) return;
        added = true; dialog.close();
      } catch (error) {
        if (current(turn)) { saving = false; status.textContent = error.message; for (const control of form.querySelectorAll('input,button')) control.disabled = false; form.elements.executable.readOnly = true; }
      }
    };
    form.querySelector('[data-close]').onclick = () => dialog.close();
    dialog.addEventListener('cancel', event => { if (form.querySelector('[data-close]').disabled) event.preventDefault(); });
    dialog.addEventListener('close', () => { alive = false; ++revision; clearTimeout(timer); if (requestId && !added) void client.request(`engine-registrations/${requestId}/cancel`, {}).catch(() => {}); dialog.remove(); resolve(added); }, { once: true });
    const header = form.querySelector('[data-drag]');
    let drag;
    header.onpointerdown = event => {
      if (event.button !== 0) return;
      const rect = dialog.getBoundingClientRect();
      drag = {x:event.clientX, y:event.clientY, left:rect.left, top:rect.top};
      dialog.style.margin = '0'; dialog.style.left = `${rect.left}px`; dialog.style.top = `${rect.top}px`;
      header.setPointerCapture?.(event.pointerId); event.preventDefault();
    };
    header.onpointermove = event => {
      if (!drag) return;
      dialog.style.left = `${Math.max(0, Math.min(window.innerWidth - dialog.offsetWidth, drag.left + event.clientX - drag.x))}px`;
      dialog.style.top = `${Math.max(0, Math.min(window.innerHeight - 50, drag.top + event.clientY - drag.y))}px`;
    };
    header.onpointerup = header.onpointercancel = () => { drag = null; };
    document.body.append(dialog); dialog.showModal(); void browse();
  });
}
