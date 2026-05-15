export function initScrollEngine() {
  const acts = document.querySelectorAll('.act-wrapper');
  const nav = document.querySelector('.nav');
  const dots = document.querySelectorAll('.progress-dot');
  const stickyCta = document.getElementById('sticky-cta');

  let audioCtx = null;
  const initAudio = () => {
    if (audioCtx) return;
    audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    if (audioCtx.state === 'suspended') audioCtx.resume();
  };

  // Multiple triggers to unlock audio on all browsers
  ['click', 'pointerdown', 'touchstart', 'keydown'].forEach(evt => 
    document.addEventListener(evt, initAudio, { once: true })
  );

  initAudio(); // Call immediately since we are now triggered by the Start button

  function playTick(isDown = true) {
    if (!audioCtx) initAudio(); // Try one last time
    if (!audioCtx || audioCtx.state === 'suspended') {
        audioCtx?.resume();
        if (audioCtx?.state === 'suspended') return; // Browser still blocking
    }
    
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.connect(gain);
    gain.connect(audioCtx.destination);
    
    osc.type = 'sine';
    const freq = isDown ? 900 : 600;
    
    osc.frequency.setValueAtTime(freq, audioCtx.currentTime);
    osc.frequency.exponentialRampToValueAtTime(freq / 3, audioCtx.currentTime + 0.12);
    
    // Increased volume for better audibility
    gain.gain.setValueAtTime(0.08, audioCtx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, audioCtx.currentTime + 0.12);
    
    osc.start();
    osc.stop(audioCtx.currentTime + 0.12);
  }

  let activeIndex = -1;
  let lastScrollY = window.scrollY;

  function onScroll() {
    const isScrollingDown = window.scrollY > lastScrollY;
    lastScrollY = window.scrollY;

    // Nav shadow
    if (window.scrollY > 60) {
      nav?.classList.add('nav--scrolled');
    } else {
      nav?.classList.remove('nav--scrolled');
    }

    // Act progress
    acts.forEach(wrapper => {
      const rect      = wrapper.getBoundingClientRect();
      const total     = wrapper.offsetHeight - window.innerHeight;
      const scrolled  = -rect.top;
      const progress  = Math.max(0, Math.min(1, scrolled / total));
      const act = wrapper.querySelector('.act');
      
      if (act) {
        act.style.setProperty('--p', progress.toFixed(4));
        
        // Parallax depth
        const layer = act.querySelector('.parallax-layer');
        if (layer) {
          layer.style.transform = `translateY(${progress * 15}vh)`;
        }
      }
        
      // Hide scroll hint
      if (wrapper.dataset.hero === "true") {
        const hint = wrapper.querySelector('.scroll-hint');
        if (hint) {
          const hintOpacity = Math.max(0, 1 - (scrolled / 300));
          hint.style.opacity = hintOpacity;
          hint.style.transform = `translateX(-50%) translateY(${scrolled * 0.2}px)`;
        }
      }
    });

    // Active dot tracking via viewport center
    let currentActiveIndex = -1;
    acts.forEach((wrapper, index) => {
      const rect = wrapper.getBoundingClientRect();
      const centerY = window.innerHeight / 2;
      if (rect.top <= centerY && rect.bottom >= centerY) {
        currentActiveIndex = index;
      }
    });

    if (currentActiveIndex !== -1 && currentActiveIndex !== activeIndex) {
      activeIndex = currentActiveIndex;
      dots.forEach((d, i) => d.classList.toggle('active', i === activeIndex));
      playTick(isScrollingDown);
    }

    // Sticky CTA
    if (stickyCta) {
      if (window.scrollY > window.innerHeight * 2.5) {
        stickyCta.classList.add('visible');
      } else {
        stickyCta.classList.remove('visible');
      }
    }

    // Reveal on scroll
    const reveals = document.querySelectorAll('.reveal');
    reveals.forEach(el => {
      const rect = el.getBoundingClientRect();
      const isVisible = rect.top < window.innerHeight * 0.85;
      if (isVisible) el.classList.add('visible');
    });
  }

  // Animation Throttling (Optimization 1)
  const actObserver = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      const act = entry.target.querySelector('.act');
      if (entry.isIntersecting) {
        act?.classList.remove('paused-anim');
      } else {
        act?.classList.add('paused-anim');
      }
    });
  }, { threshold: 0.05 });

  acts.forEach(act => actObserver.observe(act));

  window.addEventListener('scroll', onScroll, { passive: true });
  onScroll();
}
