export function initStoryChapters() {
  const demoTimer = document.getElementById('demo-timer');
  const demoStatus = document.getElementById('demo-status');
  
  const messages = [
    "Take a deep breath",
    "Stretch your body",
    "Rest your eyes for a moment",
    "Time for a quick water break"
  ];
  
  let msgIndex = 0;
  let timerSeconds = 118; // 01:58
  
  function updateTimer() {
    if (timerSeconds < 0) return;

    const mins = Math.floor(timerSeconds / 60);
    const secs = timerSeconds % 60;
    if (demoTimer) {
      demoTimer.textContent = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
    }
    
    timerSeconds--;
    
    if (timerSeconds < 0) {
      // COMPLETION STATE
      if (demoStatus) {
        demoStatus.textContent = "Congrats! Break Completed.";
        demoStatus.style.color = "var(--sky)";
        demoStatus.style.opacity = "1";
      }
      const bubble = document.getElementById('sim-bubble');
      if (bubble) bubble.style.boxShadow = "0 0 40px var(--sky)";
      clearInterval(interval);
      clearInterval(msgInterval);
      
      // Auto-reset after 5 seconds
      setTimeout(() => {
        timerSeconds = 118; // 01:58
        if (demoStatus) {
            demoStatus.style.color = "";
            demoStatus.style.opacity = "0.8";
            msgIndex = 0;
            demoStatus.textContent = messages[msgIndex];
        }
        if (bubble) bubble.style.boxShadow = "0 32px 64px -12px rgba(0, 0, 0, 0.6)";
        interval = setInterval(updateTimer, 1000);
        msgInterval = setInterval(rotateMessage, 5000);
      }, 5000);
    }
  }
  
  function rotateMessage() {
    if (!demoStatus) return;
    demoStatus.style.opacity = '0';
    setTimeout(() => {
      msgIndex = (msgIndex + 1) % messages.length;
      demoStatus.textContent = messages[msgIndex];
      demoStatus.style.opacity = '0.8';
    }, 500);
  }

  // Only run when Simulator Act is in view
  const actSim = document.getElementById('act-simulator');
  let interval;
  let msgInterval;

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        if (!interval) interval = setInterval(updateTimer, 1000);
        if (!msgInterval) msgInterval = setInterval(rotateMessage, 5000);
      } else {
        clearInterval(interval);
        clearInterval(msgInterval);
        interval = null;
        msgInterval = null;
      }
    });
  }, { threshold: 0.1 });

  if (actSim) observer.observe(actSim);
}

export function initSystemTime() {
  const timeEl = document.getElementById('system-time');
  const dayEl = document.getElementById('system-day');
  if (!timeEl || !dayEl) return;

  function update() {
    const now = new Date();
    
    // Time
    let hours = now.getHours();
    const minutes = now.getMinutes();
    const ampm = hours >= 12 ? 'PM' : 'AM';
    hours = hours % 12;
    hours = hours ? hours : 12;
    const strMinutes = minutes < 10 ? '0' + minutes : minutes;
    timeEl.innerHTML = `${hours}:${strMinutes} <i>${ampm}</i>`;

    // Day
    const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
    const dayName = days[now.getDay()];
    dayEl.innerHTML = `<i>${dayName}.</i>`;
  }

  update();
  setInterval(update, 1000 * 60); // Update every minute
}

export function initSimulator() {
  const bubble = document.getElementById('sim-bubble');
  const skipBtn = document.getElementById('sim-skip-btn');
  const text3d = document.getElementById('sim-3d-text');
  const video = document.getElementById('sim-video');
  
  const opacityInput = document.getElementById('sim-opacity');
  const opacityVal = document.getElementById('val-opacity');
  const sizeInput = document.getElementById('sim-size');
  const sizeVal = document.getElementById('val-size');
  const styleSelect = document.getElementById('sim-text-style');
  const textContentInput = document.getElementById('sim-text-content');
  const videoVolumeInput = document.getElementById('sim-video-volume');
  const videoVolumeVal = document.getElementById('val-video-volume');
  
  const modeBtns = document.querySelectorAll('.sim-btn[data-mode]');
  const styleBtns = document.querySelectorAll('.sim-btn[data-style-select]');
  
  const mediaSettings = document.getElementById('settings-media');
  const textSettings = document.getElementById('settings-text');

  if (!bubble) return;

  // Style Toggle (Media vs Text)
  styleBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      styleBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      const style = btn.dataset.styleSelect;
      
      if (style === 'Text') {
        video.style.display = 'none';
        text3d.style.display = 'block';
        mediaSettings.style.display = 'none';
        textSettings.style.display = 'block';
      } else {
        video.style.display = 'block';
        text3d.style.display = 'none';
        mediaSettings.style.display = 'block';
        textSettings.style.display = 'none';
      }
    });
  });

  // Mode Toggle (Soft vs Hard)
  modeBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      modeBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      const mode = btn.dataset.mode;
      skipBtn.style.visibility = (mode === 'Hard') ? 'hidden' : 'visible';
    });
  });

  // Proportional Scaling Function
  function updateBubbleUI() {
    const size = sizeInput.value;
    const ratio = size / 320; // 320 is our design base size
    
    bubble.style.width = `${size}px`;
    bubble.style.height = `${size}px`;
    sizeVal.textContent = `${size}px`;

    const timer = bubble.querySelector('.timer-display');
    const status = bubble.querySelector('.status-text');
    const work = bubble.querySelector('.work-status');
    const skip = bubble.querySelector('.skip-btn');

    timer.style.fontSize = `${ratio * 3.8}rem`;
    status.style.fontSize = `${ratio * 0.85}rem`;
    work.style.fontSize = `${ratio * 0.65}rem`;
    work.style.marginTop = `${ratio * 8}px`;
    skip.style.fontSize = `${ratio * 0.78}rem`;
    skip.style.padding = `${ratio * 8}px ${ratio * 24}px`;
    skip.style.marginTop = `${ratio * 24}px`;
  }

  sizeInput.addEventListener('input', updateBubbleUI);

  // Opacity
  opacityInput.addEventListener('input', (e) => {
    const val = e.target.value;
    bubble.style.backgroundColor = `rgba(255, 255, 255, ${val})`;
    opacityVal.textContent = val;
  });

  // 3D Text Content
  textContentInput.addEventListener('input', (e) => {
    text3d.textContent = e.target.value || "PAUSE";
  });

  // Video Volume
  videoVolumeInput.addEventListener('input', (e) => {
    const val = e.target.value;
    video.volume = val;
    videoVolumeVal.textContent = `${Math.round(val * 100)}%`;
  });

  // 3D Text Style
  styleSelect.addEventListener('change', (e) => {
    const style = e.target.value;
    text3d.style.animation = 'none';
    text3d.offsetHeight; // reflow
    
    if (style === 'float') text3d.style.animation = 'textFloat 8s ease-in-out infinite';
    else if (style === 'rotate') text3d.style.animation = 'textRotate 10s linear infinite';
    else if (style === 'pulse') text3d.style.animation = 'textPulse 4s ease-in-out infinite';
    else text3d.style.transform = 'rotateX(20deg) rotateY(-20deg)';
  });

  // Initial call
  updateBubbleUI();
  text3d.style.display = 'none'; // Default is Media

  // Cursor Particles in Simulator
  const actSim = document.getElementById('act-simulator');
  if (actSim) {
    actSim.addEventListener('mousemove', (e) => {
      if (Math.random() > 0.3) return; // Throttle particle creation
      const p = document.createElement('div');
      p.className = 'cursor-particle';
      p.style.left = e.clientX + 'px';
      p.style.top = e.clientY + 'px';
      document.body.appendChild(p);
      setTimeout(() => p.remove(), 800);
    });
  }
}
