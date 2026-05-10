export function initScrollReveal() {
  const observer = new IntersectionObserver(entries => {
    entries.forEach((entry, i) => {
      if (entry.isIntersecting) {
        // slight stagger if multiple elements enter at once
        entry.target.style.transitionDelay = `${i * 90}ms`;
        entry.target.classList.add('revealed');
        observer.unobserve(entry.target);
      }
    });
  }, { threshold: 0.12 });

  document.querySelectorAll('.reveal').forEach(el => observer.observe(el));

  // Also handle the hero scroll cue visibility
  const cue = document.querySelector('.scroll-cue');
  if (cue) {
    const handleScroll = () => {
      if (window.scrollY > 60) {
        cue.classList.add('hidden');
        window.removeEventListener('scroll', handleScroll);
      }
    };
    window.addEventListener('scroll', handleScroll);
  }
}