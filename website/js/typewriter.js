export function initTypewriter() {
  const terminalBody = document.querySelector('.terminal-body');
  if (!terminalBody) return;

  const script = [
    { type: 'prompt', text: 'git clone https://github.com/0xarchit/pauseCat' },
    { type: 'output', text: 'Cloning into \'pauseCat\'...' },
    { type: 'prompt', text: 'cd pauseCat' },
    { type: 'output', text: 'Read the code. Audit the architecture.' },
    { type: 'output', text: 'To run, download the MSI from Releases.' }
  ];

  let lineIndex = 0;
  let charIndex = 0;
  let currentLineEl = null;
  let isTyping = false;

  // Clear existing static content if any, except for setting up structure
  terminalBody.innerHTML = '';
  
  const createLine = (type) => {
    const div = document.createElement('div');
    if (type === 'prompt') {
      const ps = document.createElement('span');
      ps.className = 'term-prompt';
      ps.textContent = 'PS C:\\> ';
      div.appendChild(ps);
    }
    return div;
  };

  const cursor = document.createElement('span');
  cursor.className = 'term-cursor';

  const typeNextChar = () => {
    if (lineIndex >= script.length) {
      terminalBody.appendChild(cursor);
      return;
    }

    const currentLine = script[lineIndex];

    if (charIndex === 0) {
      if (currentLineEl && currentLineEl.contains(cursor)) {
        currentLineEl.removeChild(cursor);
      }
      currentLineEl = createLine(currentLine.type);
      terminalBody.appendChild(currentLineEl);
      if (currentLine.type !== 'output' && currentLine.type !== 'success') {
          currentLineEl.appendChild(cursor);
      }
    }

    if (currentLine.type === 'prompt') {
      // Type out prompt character by character
      if (charIndex < currentLine.text.length) {
        const textNode = document.createTextNode(currentLine.text.charAt(charIndex));
        currentLineEl.insertBefore(textNode, cursor);
        charIndex++;
        setTimeout(typeNextChar, 40);
      } else {
        // Line finished
        charIndex = 0;
        lineIndex++;
        setTimeout(typeNextChar, 200);
      }
    } else {
      // Instantly show output lines
      const span = document.createElement('span');
      if (currentLine.type === 'success') {
          span.className = 'term-success';
      }
      span.textContent = currentLine.text;
      currentLineEl.appendChild(span);
      
      charIndex = 0;
      lineIndex++;
      setTimeout(typeNextChar, 300);
    }
  };

  const observer = new IntersectionObserver((entries) => {
    if (entries[0].isIntersecting && !isTyping) {
      isTyping = true;
      setTimeout(typeNextChar, 500);
      observer.disconnect();
    }
  }, { threshold: 0.5 });

  observer.observe(terminalBody);
}