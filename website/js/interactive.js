export function initInteractive() {
  if (!window.matchMedia('(pointer: fine)').matches) return;

  // Create ambient lighting overlay
  const ambientGlow = document.createElement('div');
  ambientGlow.id = 'ambient-glow';
  document.body.appendChild(ambientGlow);

  window.addEventListener('mousemove', (e) => {
    // Global background ambient light tracking
    document.documentElement.style.setProperty('--mouse-x', `${e.clientX}px`);
    document.documentElement.style.setProperty('--mouse-y', `${e.clientY}px`);
  });

  ambientGlow.style.opacity = '0';
  
  window.addEventListener('mousemove', () => {
      ambientGlow.style.opacity = '1';
  }, { once: true });
}