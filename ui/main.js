const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let livingOrb = null;
try {
  const canvas = document.getElementById('byte-orb-canvas');
  const OrbClass = window.ByteLivingOrb || (typeof ByteLivingOrb !== 'undefined' ? ByteLivingOrb : null);
  if (OrbClass && canvas) {
    livingOrb = new OrbClass(canvas);
  } else {
    console.error('ByteLivingOrb class not found!');
  }
} catch (err) {
  console.error('Failed to initialize ByteLivingOrb:', err);
}

const statusText = document.getElementById('status-text');
const statusBadge = document.querySelector('.status-badge');
const presenceWidget = document.querySelector('.presence-widget');
const talkButton = document.getElementById('talk-button');
const wakeToggle = document.getElementById('wake-toggle');
const textToggle = document.getElementById('text-toggle');
const textClose = document.getElementById('text-close');
const conversationPanel = document.getElementById('conversation-panel');
const textForm = document.getElementById('text-form');
const textInput = document.getElementById('text-input');
const textSend = document.getElementById('text-send');
const textFeedback = document.getElementById('text-feedback');

let isRunning = false;
let statusResetTimer = null;
let placementMode = 'center';
let manualPlacement = false;
let orbPointerStart = null;
let orbWasDragged = false;
let suppressNextOrbClick = false;
let placementCheckPending = false;

function updateStatusBadge(state) {
  if (state !== 'error' && statusResetTimer) {
    clearTimeout(statusResetTimer);
    statusResetTimer = null;
  }
  statusBadge.className = `status-badge ${state}`;
  switch (state) {
    case 'idle':
      statusText.innerText = '';
      talkButton.setAttribute('aria-label', 'Start talking to Byte');
      talkButton.title = 'Click to talk · Ctrl+B';
      break;
    case 'listening':
      statusText.innerText = 'Listening';
      talkButton.setAttribute('aria-label', 'Stop listening to Byte');
      talkButton.title = 'Click to stop';
      break;
    case 'thinking':
      statusText.innerText = 'Thinking';
      talkButton.setAttribute('aria-label', 'Cancel Byte’s response');
      talkButton.title = 'Click to cancel';
      break;
    case 'speaking':
      statusText.innerText = 'Speaking';
      talkButton.setAttribute('aria-label', 'Stop Byte speaking');
      talkButton.title = 'Click to stop';
      break;
    case 'cancelled':
      statusText.innerText = 'Cancelled';
      talkButton.setAttribute('aria-label', 'Start talking to Byte');
      talkButton.title = 'Click to talk · Ctrl+B';
      break;
    case 'error':
      statusText.innerText = 'Couldn’t complete · click to retry';
      talkButton.setAttribute('aria-label', 'Retry talking to Byte');
      talkButton.title = 'Click to try again';
      break;
    case 'hidden':
      presenceWidget.classList.add('hidden');
      return;
  }
  presenceWidget.classList.remove('hidden');
}

function updateWakeToggle(enabled) {
  wakeToggle.textContent = `Wake word: ${enabled ? 'On' : 'Off'}`;
  wakeToggle.setAttribute('aria-pressed', String(enabled));
  wakeToggle.setAttribute('aria-busy', 'false');
  wakeToggle.setAttribute(
    'aria-label',
    `Wake word detection is ${enabled ? 'on' : 'off'}. Click to turn it ${enabled ? 'off' : 'on'}.`
  );
  wakeToggle.title = `Click to turn wake word detection ${enabled ? 'off' : 'on'}`;
}

async function setConversationOpen(open) {
  const win = window.__TAURI__.window.getCurrentWindow();
  if (open) {
    conversationPanel.hidden = false;
    document.body.classList.add('text-open');
    textToggle.setAttribute('aria-expanded', 'true');
    try {
      await win.show();
      await invoke('set_conversation_window_size', { expanded: true, placementMode: manualPlacement ? 'manual' : placementMode });
      await win.setFocus();
      textInput.focus();
    } catch (err) {
      console.error('Failed to open conversation panel:', err);
      conversationPanel.hidden = true;
      document.body.classList.remove('text-open');
      textToggle.setAttribute('aria-expanded', 'false');
    }
  } else {
    conversationPanel.hidden = true;
    document.body.classList.remove('text-open');
    textToggle.setAttribute('aria-expanded', 'false');
    try {
      await invoke('set_conversation_window_size', { expanded: false, placementMode: manualPlacement ? 'manual' : placementMode });
    } catch (err) {
      console.error('Failed to close conversation panel:', err);
    }
  }
}

textToggle.addEventListener('click', () => setConversationOpen(true));
textClose.addEventListener('click', () => setConversationOpen(false));

textForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const text = textInput.value.trim();
  if (!text || isRunning) return;

  textInput.value = '';
  textSend.disabled = true;
  textFeedback.textContent = 'Thinking…';
  isRunning = true;
  try {
    const interaction = invoke('start_text_interaction', { text });
    await setConversationOpen(false);
    await interaction;
    textFeedback.textContent = 'Byte has replied aloud.';
  } catch (err) {
    console.error('Text interaction failed:', err);
    textFeedback.textContent = String(err);
  } finally {
    isRunning = false;
    textSend.disabled = false;
  }
});

textInput.addEventListener('keydown', (event) => {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    textForm.requestSubmit();
  }
});

wakeToggle.addEventListener('click', async () => {
  const enabled = wakeToggle.getAttribute('aria-pressed') !== 'true';
  wakeToggle.disabled = true;
  try {
    updateWakeToggle(await invoke('set_wake_word_enabled', { enabled }));
  } catch (err) {
    console.error('Failed to change wake word setting:', err);
    wakeToggle.textContent = 'Wake word: Error';
  } finally {
    wakeToggle.disabled = false;
  }
});

invoke('get_wake_word_enabled')
  .then((enabled) => {
    updateWakeToggle(enabled);
    wakeToggle.disabled = false;
  })
  .catch((err) => {
    console.error('Failed to load wake word setting:', err);
    wakeToggle.textContent = 'Wake word: Unavailable';
    wakeToggle.disabled = true;
  });

// Full conversational lifecycle run
async function runInteraction() {
  if (isRunning) return;
  if (statusResetTimer) {
    clearTimeout(statusResetTimer);
    statusResetTimer = null;
  }
  isRunning = true;

  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    const win = getCurrentWindow();
    await win.show();
    await win.setFocus();
  } catch (err) {
    console.warn('Window focus/show notice:', err);
  }

  let result = null;
  let failed = false;
  try {
    result = await invoke('start_interaction');
    console.log('Interaction turn completed:', result);
    if (result) {
    }

    if (result && result.auto_listen) {
      setTimeout(() => {
        isRunning = false;
        runInteraction();
      }, 400);
      return;
    }
  } catch (err) {
    console.error('Interaction loop error:', err);
    failed = true;
  } finally {
    isRunning = false;
    if (livingOrb) livingOrb.setAudioLevel(0.0);
    if (!result || !result.auto_listen) {
      if (failed) {
        if (livingOrb) livingOrb.setState('idle');
        updateStatusBadge('error');
        statusResetTimer = setTimeout(() => updateStatusBadge('idle'), 3000);
      } else {
        if (livingOrb) livingOrb.setState('idle');
        updateStatusBadge('idle');
      }
    }
  }
}

// Click orb to speak or stop
talkButton.addEventListener('pointerdown', (event) => {
  if (event.button !== 0) return;
  orbPointerStart = { x: event.screenX, y: event.screenY };
  orbWasDragged = false;
});

window.addEventListener('pointermove', async (event) => {
  if (!orbPointerStart || orbWasDragged) return;
  const dx = event.screenX - orbPointerStart.x;
  const dy = event.screenY - orbPointerStart.y;
  if (Math.hypot(dx, dy) < 7) return;

  orbWasDragged = true;
  manualPlacement = true;
  try {
    await window.__TAURI__.window.getCurrentWindow().startDragging();
  } catch (err) {
    console.error('Could not drag Byte window:', err);
    manualPlacement = false;
    orbWasDragged = false;
    orbPointerStart = null;
  }
});

window.addEventListener('pointerup', () => {
  if (orbWasDragged) {
    suppressNextOrbClick = true;
    setTimeout(() => { suppressNextOrbClick = false; }, 600);
  }
  orbPointerStart = null;
  orbWasDragged = false;
});

window.addEventListener('pointercancel', () => {
  orbPointerStart = null;
  orbWasDragged = false;
});

talkButton.addEventListener('click', async () => {
  if (suppressNextOrbClick) {
    suppressNextOrbClick = false;
    return;
  }
  if (!isRunning) {
    runInteraction();
  } else {
    try {
      await invoke('stop_action');
    } catch (err) {
      console.error('Failed to cancel interaction on orb click:', err);
    }
  }
});

// Tauri Event Subscriptions
listen('presence_state_changed', (event) => {
  const state = event.payload;
  if (livingOrb) livingOrb.setState(state);
  updateStatusBadge(state);
});

listen('audio_level', (event) => {
  const level = typeof event.payload === 'number' ? event.payload : 0.0;
  if (livingOrb) livingOrb.setAudioLevel(level);
});

listen('wakeup', () => {
  console.log('Byte wakeup signal received!');
  runInteraction();
});

// Keyboard controls: Escape to dismiss/cancel, Ctrl+B toggle
window.addEventListener('keydown', async (event) => {
  if (event.key === 'Escape') {
    event.preventDefault();
    if (document.body.classList.contains('text-open') && !isRunning) {
      await setConversationOpen(false);
      return;
    }
    try {
      await invoke('stop_action');
    } catch (err) {
      console.error('Escape stop error:', err);
    }
  } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'b') {
    event.preventDefault();
    if (!isRunning) {
      runInteraction();
    } else {
      await invoke('stop_action');
    }
  }
});

async function updatePresencePlacement() {
  if (manualPlacement || isRunning || document.body.classList.contains('text-open') || placementCheckPending) return;
  placementCheckPending = true;
  try {
    placementMode = await invoke('reposition_presence', { currentMode: placementMode });
  } catch (err) {
    console.error('Could not position Byte:', err);
  } finally {
    placementCheckPending = false;
  }
}

window.addEventListener('DOMContentLoaded', updatePresencePlacement);

setInterval(updatePresencePlacement, 2500);

// Initial state setup
updateStatusBadge('idle');
if (livingOrb) livingOrb.setState('idle');
