export function initInteractive() {
  if (!window.matchMedia('(pointer: fine)').matches) return;

  // Create cursor elements
  const cursor = document.createElement('div');
  cursor.id = 'custom-cursor';
  
  const cursorGlow = document.createElement('div');
  cursorGlow.id = 'custom-cursor-glow';
  
  // Create ambient lighting overlay
  const ambientGlow = document.createElement('div');
  ambientGlow.id = 'ambient-glow';
  
  document.body.appendChild(ambientGlow);
  document.body.appendChild(cursorGlow);
  document.body.appendChild(cursor);

  let mouseX = window.innerWidth / 2;
  let mouseY = window.innerHeight / 2;
  let cursorX = mouseX;
  let cursorY = mouseY;
  let glowX = mouseX;
  let glowY = mouseY;

  const cursorSpeed = 0.5;
  const glowSpeed = 0.15;

  const animate = () => {
    cursorX += (mouseX - cursorX) * cursorSpeed;
    cursorY += (mouseY - cursorY) * cursorSpeed;
    glowX += (mouseX - glowX) * glowSpeed;
    glowY += (mouseY - glowY) * glowSpeed;

    cursor.style.transform = `translate3d(${cursorX}px, ${cursorY}px, 0)`;
    cursorGlow.style.transform = `translate3d(${glowX}px, ${glowY}px, 0)`;

    requestAnimationFrame(animate);
  };

  window.addEventListener('mousemove', (e) => {
    mouseX = e.clientX;
    mouseY = e.clientY;
    
    // Global background ambient light tracking
    document.documentElement.style.setProperty('--mouse-x', `${e.clientX}px`);
    document.documentElement.style.setProperty('--mouse-y', `${e.clientY}px`);
  });

  // Track hover states for interactive elements
  const updateInteractables = () => {
    const interactables = document.querySelectorAll('a, button, .card, .chip, .btn-ghost, .nav-logo');
    interactables.forEach(el => {
      if (el.dataset.cursorBound) return;
      el.dataset.cursorBound = "true";
      
      el.addEventListener('mouseenter', () => {
        cursor.classList.add('hovering');
        cursorGlow.classList.add('hovering');
      });
      el.addEventListener('mouseleave', () => {
        cursor.classList.remove('hovering');
        cursorGlow.classList.remove('hovering');
      });
    });
  };

  updateInteractables();
  
  // Ensure cursors don't show off-screen initially
  cursor.style.opacity = '0';
  cursorGlow.style.opacity = '0';
  ambientGlow.style.opacity = '0';
  
  window.addEventListener('mousemove', () => {
      cursor.style.opacity = '1';
      cursorGlow.style.opacity = '1';
      ambientGlow.style.opacity = '1';
  }, { once: true });

  animate();
}