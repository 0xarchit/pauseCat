export function initNav() {
  const nav = document.querySelector('nav');
  
  window.addEventListener('scroll', () => {
    if (window.scrollY > 60) {
      nav.classList.add('scrolled');
    } else {
      nav.classList.remove('scrolled');
    }
  });

  // Add mobile menu logic here if needed later.
  // Currently sticking to the core desktop spec but ready for extension.
}