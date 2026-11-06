import { matchingStartupException, submitBugReport } from './analytics.js';

const labels = {
  app_build: 'App build',
  browser: 'Browser',
  os: 'Operating system',
  display_mode: 'Display mode',
  area: 'Area',
  startup_stage: 'Startup stage',
  failure_reason: 'Failure reason',
  http_status: 'HTTP status',
  posthog_exception_url: 'Related PostHog error (project access required)',
  posthog_exception_id: 'Related error reference',
};

function browserDiagnostics() {
  const ua = navigator.userAgent;
  const browser = ua.match(/(Edg|OPR)\/(\d+)/) ??
    ua.match(/(Firefox|Chrome)\/(\d+)/) ??
    ua.match(/(Version)\/(\d+)[^]*Safari\//);
  const os = ua.match(/(Windows NT|Mac OS X|Android) ([\d._]+)/);
  const browserNames = { Edg: 'Edge', OPR: 'Opera', Version: 'Safari' };
  return {
    browser: browser ? `${browserNames[browser[1]] ?? browser[1]} ${browser[2]}` : 'Unknown',
    os: os ? `${os[1]} ${os[2].replaceAll('_', '.')}` : /Linux/.test(ua) ? 'Linux' : 'Unknown',
    display_mode: matchMedia('(display-mode: standalone)').matches ? 'Installed app' : 'Browser tab',
  };
}

export function openBugReport(startup, pgn) {
  if (document.querySelector('#ironwood-bug-report')) return;
  const dialog = document.createElement('dialog');
  dialog.id = 'ironwood-bug-report';
  dialog.setAttribute('aria-labelledby', 'bug-report-title');
  dialog.setAttribute('aria-describedby', 'bug-report-intro');
  const canAttach = Boolean(pgn) && new TextEncoder().encode(pgn).length <= 60 * 1024;
  dialog.innerHTML = `
    <header>
      <span class="report-eyebrow">Ironwood support</span>
      <h2 id="bug-report-title">Report a bug</h2>
      <p id="bug-report-intro">Tell us what went wrong so we can reproduce it.</p>
    </header>
    <form>
      <label class="report-field">What happened?
        <textarea name="description" rows="4" maxlength="4000" required autofocus
          placeholder="Include the steps you took and what you expected to happen."></textarea>
      </label>
      ${pgn ? `<section class="report-attachment">
        <label><input type="checkbox" name="attachPgn" aria-describedby="report-pgn-note" ${canAttach ? '' : 'disabled'}>
          Attach current game <span class="report-file-type">PGN</span></label>
        <p id="report-pgn-note">${canAttach ? 'Includes moves and any player names in the export.' : 'This game exceeds the 60 KB attachment limit.'}</p>
        ${canAttach ? '<details hidden><summary>Preview game PGN</summary><pre class="report-pgn"></pre></details>' : ''}
      </section>` : ''}
      <label class="report-field"><span>Reply email <small>Optional</small></span>
        <input type="email" name="email" maxlength="254" autocomplete="email" placeholder="you@example.com">
      </label>
      <details class="report-diagnostics">
        <summary>View included diagnostics</summary>
        <pre></pre>
      </details>
      <p class="report-disclosure">Your report, including any attachment, goes to Ironwood through PostHog. Your analytics preference stays the same. Please leave out passwords and connection tokens.</p>
      <p role="status"></p>
      <footer>
        <button type="button">Cancel</button>
        <button type="submit">Send report</button>
      </footer>
    </form>`;
  const form = dialog.querySelector('form');
  const preview = dialog.querySelector('.report-diagnostics pre');
  const attachment = form.elements.attachPgn;
  if (canAttach) {
    const pgnPreview = dialog.querySelector('.report-pgn');
    pgnPreview.textContent = pgn;
    attachment.addEventListener('change', () => {
      pgnPreview.closest('details').hidden = !attachment.checked;
    });
  }
  const status = dialog.querySelector('[role="status"]');
  const send = dialog.querySelector('[type="submit"]');
  const close = dialog.querySelector('[type="button"]');
  const base = {
    app_build: '__PACKAGE_VERSION__'.startsWith('__') ? 'Development' : '__PACKAGE_VERSION__',
    ...browserDiagnostics(),
    area: startup ? 'Startup' : 'Chess app',
    ...startup,
  };
  const reportId = crypto.randomUUID();
  let diagnostics;
  const refresh = () => {
    const current = { ...base, ...(startup ? matchingStartupException() : undefined) };
    const changed = JSON.stringify(current) !== JSON.stringify(diagnostics);
    diagnostics = current;
    preview.textContent = Object.entries(current).map(([key, value]) => `${labels[key]}: ${value}`).join('\n');
    return changed;
  };
  refresh();
  close.onclick = () => dialog.close();
  dialog.addEventListener('cancel', event => { if (close.disabled) event.preventDefault(); });
  dialog.addEventListener('close', () => dialog.remove());
  form.addEventListener('submit', async event => {
    event.preventDefault();
    if (send.disabled) return;
    const description = form.elements.description.value.trim();
    if (!description) {
      status.textContent = 'Describe what happened before sending.';
      return;
    }
    if (refresh()) {
      preview.closest('details').open = true;
      status.textContent = 'The diagnostics changed. Review them before sending.';
      return;
    }
    const email = form.elements.email.value.trim();
    const includePgn = canAttach && attachment.checked;
    if (attachment) attachment.disabled = true;
    send.disabled = close.disabled = true;
    status.textContent = 'Sending report…';
    try {
      await submitBugReport(reportId, {
        description,
        ...(email ? { reply_email: email } : {}),
        ...diagnostics,
        ...(includePgn ? { pgn } : {}),
      });
      form.elements.description.readOnly = form.elements.email.readOnly = true;
      status.textContent = `Report sent. Reference: ${reportId}`;
      close.textContent = 'Close';
    } catch {
      status.textContent = 'Could not send the report. Check your connection and try again.';
      send.disabled = false;
      if (canAttach) attachment.disabled = false;
    } finally {
      close.disabled = false;
    }
  });
  document.body.append(dialog);
  dialog.showModal();
}
