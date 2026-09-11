const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const canvas = document.getElementById('byte-orb-canvas');
const livingOrb = new window.ByteLivingOrb(canvas);

const statusText = document.getElementById('status-text');
const statusBadge = document.querySelector('.status-badge');
const presenceWidget = document.querySelector('.presence-widget');
const orbViewport = document.querySelector('.orb-viewport');

let isRunning = false;

function updateStatusBadge(state) {
  statusBadge.className = `status-badge ${state}`;
  switch (state) {
    case 'idle':
      statusText.innerText = 'Click or Ctrl+B';
      break;
    case 'listening':
      statusText.innerText = 'Listening...';
      break;
    case 'thinking':
      statusText.innerText = 'Thinking...';
      break;
    case 'speaking':
      statusText.innerText = 'Speaking...';
      break;
    case 'cancelled':
      statusText.innerText = 'Cancelled';
      break;
    case 'hidden':
      presenceWidget.classList.add('hidden');
      return;
  }
  presenceWidget.classList.remove('hidden');
}

// Full conversational lifecycle run
async function runInteraction() {
  if (isRunning) return;
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
  try {
    result = await invoke('start_interaction');
    console.log('Interaction turn completed:', result);

    if (result && result.auto_listen) {
      setTimeout(() => {
        isRunning = false;
        runInteraction();
      }, 400);
      return;
    }
  } catch (err) {
    console.error('Interaction loop error:', err);
  } finally {
    isRunning = false;
    livingOrb.setAudioLevel(0.0);
    if (!result || !result.auto_listen) {
      livingOrb.setState('idle');
      updateStatusBadge('idle');
    }
  }
}

// Click orb to speak or stop
orbViewport.addEventListener('click', async () => {
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
  livingOrb.setState(state);
  updateStatusBadge(state);
});

listen('audio_level', (event) => {
  const level = typeof event.payload === 'number' ? event.payload : 0.0;
  livingOrb.setAudioLevel(level);
});

listen('wakeup', () => {
  console.log('Byte wakeup signal received!');
  runInteraction();
});

// Keyboard controls: Escape to dismiss/cancel, Ctrl+B toggle
window.addEventListener('keydown', async (event) => {
  if (event.key === 'Escape') {
    event.preventDefault();
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

// Cursor ignore click-through outside orb bounds
orbViewport.addEventListener('mouseenter', async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(false);
  } catch (_) {}
});

orbViewport.addEventListener('mouseleave', async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(true);
  } catch (_) {}
});

window.addEventListener('DOMContentLoaded', async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(true);
    await invoke('reposition_presence');
  } catch (_) {}
});

// Initial state setup
updateStatusBadge('idle');
livingOrb.setState('idle');
