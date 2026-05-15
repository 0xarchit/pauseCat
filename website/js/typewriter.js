export function initTypewriter() {
  const terminal = document.querySelector('.terminal-body');
  if (!terminal) return;

  const lines = [
    { text: 'PS C:\\> ', type: 'prompt' },
    { text: 'git clone https://github.com/0xarchit/pauseCat', type: 'text' },
    { text: '\nPS C:\\> ', type: 'prompt' },
    { text: 'cd pauseCat', type: 'text' },
    { text: '\nPS C:\\> ', type: 'prompt' },
    { text: 'cargo build --release', type: 'text' },
    { text: '\n\nCompiling pausecat v1.1.2', type: 'text' },
    { text: '\nFinished release [optimized] in 18.4s', type: 'success' },
    { text: '\n\nPS C:\\> ', type: 'prompt' },
    { text: '.\\target\\release\\pausecat.exe', type: 'text' },
    { text: '\n[PauseCat] Tray initialized. Timer started. ', type: 'text' },
    { text: '█', type: 'cursor' }
  ];

  let lineIdx = 0;
  let charIdx = 0;
  terminal.innerHTML = '';

  function type() {
    if (lineIdx >= lines.length) return;

    const line = lines[lineIdx];
    if (line.type === 'cursor') {
        const span = document.createElement('span');
        span.className = 't-cursor';
        terminal.appendChild(span);
        return;
    }

    if (charIdx === 0) {
      const span = document.createElement('span');
      if (line.type === 'prompt') span.className = 't-prompt';
      if (line.type === 'success') span.className = 't-success';
      terminal.appendChild(span);
    }

    const currentSpan = terminal.lastChild;
    const char = line.text[charIdx];
    
    if (char === '\n') {
        terminal.appendChild(document.createElement('br'));
    } else {
        currentSpan.textContent += char;
    }

    charIdx++;
    if (charIdx >= line.text.length) {
      charIdx = 0;
      lineIdx++;
      setTimeout(type, 200);
    } else {
      setTimeout(type, 35);
    }
  }

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        type();
        observer.unobserve(terminal);
      }
    });
  }, { threshold: 0.5 });

  observer.observe(terminal);
}
