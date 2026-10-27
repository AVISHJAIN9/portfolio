/**
 * glassPortfolio.js — Cinematic Glassmorphism Portfolio Layer
 *
 * Exact Scene-to-Content Placement:
 *  - Chapter 4: S7_REINDEER (Hollow Lantern Tree) -> ABOUT ME + CONTACT overlay
 *  - Chapter 5: S9_ICE_CAVE (Glacial Vault)       -> ACHIEVEMENTS (exact same copy) + NEXT BUTTON
 *  - Chapter 6: S8_HOLD (Alpine Summit)          -> Free Exploration (No overlay; eagle & reindeer)
 *  - Scene 10:  S10_LOOP (Mountain Summit Loop)  -> PROJECTS (triggers when scene 10 loops)
 *  - Chapter 7: S_HOUSE (House Exterior)         -> Free Exploration (No overlay; click door to enter)
 *  - Scene 12:  S12_INTERIOR (Craftsman's Table) -> SKILLS (triggers at end of scene 12 table closeup)
 *  - Scene 13:  S13_FINALE (Celestial Aurora)     -> STARTUP above CONTACT on left; AURORA MENTION above CLUBS on right
 *  - Scene 14:  S14_CLUBS (Clubs Showcase)        -> 4-Video Showcase (unchanged)
 */

(function () {
  'use strict';

  class GlassPortfolio {
    constructor() {
      this.data = {
        profile: null,
        certificates: null,
        projects: null,
        skills: null,
        startup: null,
        upcoming: null,
        blog: null,
      };

      this.currentProjectFilter = 'all';
      this.projectSearchQuery   = '';
      this.activeCertificates   = [];
      this.lightboxIndex        = 0;
      this.autoScrollPaused     = false;
      this.autoScrollAnimId     = null;
      this.statsAnimated        = false;
      this.finaleSubPanel       = null;
      this.activePanelId        = null;
    }

    async init() {
      console.log('[GLASS] Initializing GlassPortfolio Engine...');
      await this.loadAllContent();
      this.renderAllPanels();
      this.setupGlobalInteractions();
      this.setupA3TiltAndSpotlight();
      this.setupCertificatesAutoScroll();
      this.setupLightbox();
      this.setupPDFViewer();

      // Check current state from app if already set
      if (window.State && window.State.currentState) {
        this.onStateChange(window.State.currentState);
      }
      console.log('[GLASS] Engine Ready with decoupled JSON content.');
    }

    async loadAllContent() {
      const files = ['profile', 'certificates', 'projects', 'skills', 'startup', 'upcoming', 'blog'];
      const results = await Promise.all(
        files.map(f =>
          fetch(`content/${f}.json`)
            .then(r => (r.ok ? r.json() : null))
            .catch(err => {
              console.warn(`[GLASS] Could not load content/${f}.json:`, err);
              return null;
            })
        )
      );
      files.forEach((name, i) => {
        this.data[name] = results[i] || [];
      });
    }

    renderAllPanels() {
      let c = document.getElementById('glass-stage-container');
      if (!c) {
        c = document.createElement('div');
        c.id = 'glass-stage-container';
        c.setAttribute('aria-live', 'polite');
        document.body.appendChild(c);
      }
      c.innerHTML = this._html();
    }

    _html() {
      return `
        <!-- Panel Scrim: dark dim layer behind panels (strictly NO blur) -->
        <div class="panel-scrim" aria-hidden="true"></div>

        <!-- Chapter 4: Hollow Lantern Tree -> About Me + Contact -->
        <section id="panel-intro" class="glass-page-layer glass-panel" style="display:none;" aria-label="About Me and Contact">
          ${this._intro()}
        </section>

        <!-- Chapter 5: Glacial Vault -> Achievements & Certificates -->
        <section id="panel-achievements" class="glass-page-layer glass-panel" style="display:none;" aria-label="Achievements">
          ${this._achievements()}
        </section>

        <!-- Scene 10: Mountain Peak Flight -> Projects -->
        <section id="panel-projects" class="glass-page-layer glass-panel" style="display:none;" aria-label="Projects">
          ${this._projects()}
        </section>

        <!-- Scene 12: Craftsman's Table Workbench -> Skills -->
        <section id="panel-skills" class="glass-page-layer glass-panel" style="display:none;" aria-label="Skills">
          ${this._skills()}
        </section>

        <!-- Scene 13: Celestial Aurora -> Startup + Contact (Left) & Aurora Mention + 3 Buttons (Right) -->
        <div id="panel-finale" class="glass-page-layer finale-layer" style="display:none;" aria-label="Celestial Aurora Showcase">
          ${this._finale()}
        </div>

        <!-- Sub-Panels (Blog, Upcoming Events, Future Projects) -->
        <section id="panel-blog" class="glass-page-layer glass-panel sub-panel" style="display:none;" aria-label="Blog">
          ${this._blog()}
        </section>
        <section id="panel-upcoming" class="glass-page-layer glass-panel sub-panel" style="display:none;" aria-label="Upcoming Events">
          ${this._upcoming()}
        </section>
        <section id="panel-future-projects" class="glass-page-layer glass-panel sub-panel" style="display:none;" aria-label="Future Projects / Coming Soon">
          ${this._futureProjects()}
        </section>

        <div id="glass-toast" class="glass-toast" aria-live="polite" role="status">Email copied</div>

        <!-- Certificate Lightbox -->
        <div id="cert-lightbox" class="glass-lightbox" aria-hidden="true" role="dialog" aria-modal="true" style="display:none;">
          <div class="lightbox-backdrop"></div>
          <button class="lightbox-close icon-btn" aria-label="Close">&#x2715;</button>
          <button class="lightbox-prev icon-btn" aria-label="Previous">&#x276E;</button>
          <button class="lightbox-next icon-btn" aria-label="Next">&#x276F;</button>
          <div class="lightbox-stage"><img id="lightbox-img" class="lightbox-image" src="" alt="Certificate"/></div>
          <div class="lightbox-bar">
            <div id="lightbox-caption" class="lightbox-caption"></div>
            <a id="lightbox-download" class="glass-pill-btn download-btn" href="#" download target="_blank" rel="noopener noreferrer">&#x2B07; Download original</a>
          </div>
        </div>

        <!-- PDF Modal -->
        <div id="pdf-modal" class="glass-pdf-modal" aria-hidden="true" role="dialog" aria-modal="true" style="display:none;">
          <div class="pdf-modal-backdrop"></div>
          <div class="pdf-modal-dialog">
            <header class="pdf-modal-header">
              <span id="pdf-modal-title" class="pdf-modal-heading">Document Viewer</span>
              <div class="pdf-modal-actions">
                <a id="pdf-modal-download" class="glass-pill-btn download-btn" href="#" download target="_blank" rel="noopener noreferrer">Download PDF</a>
                <button id="pdf-modal-close" class="icon-btn" aria-label="Close">&#x2715;</button>
              </div>
            </header>
            <div class="pdf-modal-body"><iframe id="pdf-modal-iframe" src="" title="PDF Document"></iframe></div>
          </div>
        </div>`;
    }

    /* ============================================================
       1. ABOUT ME & CONTACT (S7_REINDEER — Chapter 4: Hollow Lantern Tree)
       ============================================================ */
    _intro() {
      const p = this.data.profile || {};
      const name = p.name || 'Avish Jain';
      const tagline = p.tagline || 'Building backend systems that hold up when the weather turns.';
      const subline = p.subline || '2nd-year Engineering student, BMS College of Engineering \u00b7 Founder, All Tech';
      const bio = p.bio || 'Focused on backend and distributed systems; building 20 systems-level projects from first principles.';
      const parents = p.parents || 'son of Ritu & Ankush Jain';
      const college = p.college || '2nd year, BMS College of Engineering, Bengaluru';
      const contact = p.contact || {
        email: 'avishjain412@gmail.com',
        linkedin: 'https://www.linkedin.com/in/avish-jain-956962362',
        github: 'https://github.com/AVISHJAIN9'
      };
      const quote = p.quote || 'Simplicity is prerequisite for reliability.';

      return `
        <div class="panel-frost-sparkles" aria-hidden="true">
          <span class="sparkle s1">&#x2726;</span><span class="sparkle s2">&#x2726;</span>
          <span class="sparkle s3">&#x2726;</span><span class="sparkle s4">&#x2726;</span>
        </div>
        <header class="glass-panel-header sticky-header">
          <div>
            <h2 class="panel-main-title">About Me &amp; Contact</h2>
            <p class="panel-subtitle-text">${name} &mdash; ${subline}</p>
          </div>
          <button class="icon-btn panel-close-btn" data-close-panel aria-label="Close">&#x2715;</button>
        </header>
        <div class="glass-panel-content">
          <div class="about-grid">
            <!-- Left Card: Bio, Philosophy & Affiliations -->
            <div class="glass-card about-main-card frost-edge-border">
              <span class="highlight-badge">ENGINEERING PHILOSOPHY</span>
              <h3 class="about-tagline">&ldquo;${tagline}&rdquo;</h3>
              <p class="about-bio">${bio}</p>

              <div class="about-meta-row">
                <div class="about-meta-pill">
                  <span class="meta-label">Affiliation</span>
                  <span class="meta-val">${college}</span>
                </div>
                <div class="about-meta-pill">
                  <span class="meta-label">Personal Note</span>
                  <span class="meta-val">${parents}</span>
                </div>
              </div>

              <div class="about-focus-tags">
                <span class="tag-chip lang-chip">Distributed Systems</span>
                <span class="tag-chip lang-chip">Low-Latency Networking</span>
                <span class="tag-chip lang-chip">Concurrency &amp; Netcode</span>
                <span class="tag-chip lang-chip">Operating Systems</span>
                <span class="tag-chip lang-chip">Bytecode VMs &amp; GC</span>
              </div>
            </div>

            <!-- Right Card: Direct Contact & Quick Stats -->
            <div class="about-side-column">
              <div class="glass-card contact-card frost-edge-border">
                <span class="highlight-badge">CONNECT &amp; INQUIRE</span>
                <h4 class="contact-card-title">Direct Contact</h4>
                <div class="contact-pill-item">
                  <span class="contact-label">Email</span>
                  <a href="mailto:${contact.email}" class="contact-val">${contact.email}</a>
                  <button class="glass-pill-btn copy-email-btn" data-email="${contact.email}" aria-label="Copy email">Copy</button>
                </div>
                <div class="contact-pill-item">
                  <span class="contact-label">LinkedIn</span>
                  <span class="contact-val">Avish Jain</span>
                  <a href="${contact.linkedin}" class="glass-pill-btn" target="_blank" rel="noopener noreferrer">Connect &#x2197;</a>
                </div>
                <div class="contact-pill-item">
                  <span class="contact-label">GitHub</span>
                  <span class="contact-val">AVISHJAIN9</span>
                  <a href="${contact.github}" class="glass-pill-btn" target="_blank" rel="noopener noreferrer">Follow &#x2197;</a>
                </div>
                <div class="contact-quote-box">
                  <span class="quote-text">&ldquo;${quote}&rdquo;</span>
                </div>
              </div>

              <div class="about-stats-row">
                <div class="glass-card stat-tile">
                  <span class="stat-number">20</span>
                  <span class="stat-label">Distributed Engines</span>
                </div>
                <div class="glass-card stat-tile">
                  <span class="stat-composite">2<span class="stat-suffix">&times;</span></span>
                  <span class="stat-label">SIH R2 Qualified</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Bottom Action Bar to Continue to Glacial Cave -->
          <div class="intro-action-footer">
            <p class="intro-hint-text">The amber lantern glows beneath the spirit reindeer. Ready to enter the glacial vault?</p>
            <button id="btn-intro-continue" class="glass-pill-btn highlight-btn" aria-label="Venture into the Glacial Cave">
              Venture into the Glacial Cave &rarr;
            </button>
          </div>
        </div>`;
    }

    /* ============================================================
       2. ACHIEVEMENTS (S9_ICE_CAVE — Chapter 5: Glacial Vault)
       ============================================================ */
    _achievements() {
      const certs = (this.data.certificates || []).filter(c => c.image);
      return `
        <div class="panel-frost-sparkles" aria-hidden="true">
          <span class="sparkle s1">&#x2726;</span><span class="sparkle s2">&#x2726;</span>
          <span class="sparkle s3">&#x2726;</span><span class="sparkle s4">&#x2726;</span>
          <span class="sparkle s5">&#x2726;</span><span class="sparkle s6">&#x2726;</span>
        </div>
        <header class="glass-panel-header sticky-header">
          <div>
            <h2 class="panel-main-title">Achievements</h2>
            <p class="panel-subtitle-text">Hackathons qualified, credentials verified, competitive milestones</p>
          </div>
          <button class="icon-btn panel-close-btn" data-close-panel aria-label="Close Achievements">&#x2715;</button>
        </header>
        <div class="glass-panel-content">

          <!-- 2A Highlight Cards (Exact Copy) -->
          <div class="highlights-row">
            <div class="glass-card highlight-card frost-edge-border">
              <span class="highlight-badge">HACKATHON MILESTONE</span>
              <h3 class="highlight-title">Smart India Hackathon 2025</h3>
              <p class="highlight-desc">Qualified for Round 2 &middot; Problem Statement 25008</p>
            </div>
            <div class="glass-card highlight-card frost-edge-border">
              <div class="highlight-top">
                <span class="highlight-badge">HACKATHON MILESTONE</span>
                <span class="status-chip chip-breathing"><span class="breathing-dot"></span>Further results awaited</span>
              </div>
              <h3 class="highlight-title">Smart India Hackathon 2026</h3>
              <p class="highlight-desc">Qualified for Round 2 &middot; Project SAATHI (SIH26107)</p>
            </div>
            <div class="glass-card highlight-card frost-edge-border">
              <span class="highlight-badge">NATIONAL EXHIBITION</span>
              <h3 class="highlight-title">Top 35 Teams</h3>
              <p class="highlight-desc">Dr. B.R. Ambedkar 10th National Techno Exhibition</p>
            </div>
          </div>

          <!-- 2B Certificate Gallery Section -->
          <div class="section-divider">
            <h3 class="section-heading">Verified Credentials &amp; Certifications</h3>
            <div class="cert-filter-chips" role="tablist" aria-label="Filter certificates">
              <button class="glass-chip active" data-cert-filter="all">All</button>
              <button class="glass-chip" data-cert-filter="Hackathons &amp; Competitions">Hackathons &amp; Competitions</button>
              <button class="glass-chip" data-cert-filter="Workshops &amp; Learning">Workshops &amp; Learning</button>
              <button class="glass-chip" data-cert-filter="Volunteer &amp; Leadership">Volunteer &amp; Leadership</button>
            </div>
          </div>

          <div class="cert-marquee-container" id="cert-marquee-container">
            <div class="cert-marquee-mask-top" aria-hidden="true"></div>
            <div class="cert-marquee-track" id="cert-marquee-track">
              ${this._certCards(certs)}
              ${this._certCards(certs, true)}
            </div>
            <div class="cert-marquee-mask-bottom" aria-hidden="true"></div>
          </div>

          <!-- 2C Live Stat Tile & Next Continuation Button -->
          <div class="achievements-footer-row">
            <div class="glass-card stat-tile cert-stat-tile">
              <span class="stat-number" data-count-to="${certs.length}">0</span>
              <span class="stat-label">certificates earned</span>
            </div>

            <div class="cave-continue-box">
              <span class="cave-continue-hint">Ready to ascend from the icy cavern to the alpine heights?</span>
              <button id="btn-cave-continue" class="glass-pill-btn highlight-btn cave-continue-btn" aria-label="Continue to Alpine Summit">
                Continue to Alpine Summit &rarr;
              </button>
            </div>
          </div>

        </div>`;
    }

    _certCards(certs, isDupe = false) {
      return certs.map((c, idx) => {
        const imgSrc  = c.image;
        const caption = [c.title, c.detail, c.role].filter(Boolean).join(' \u00b7 ');
        const aria    = isDupe ? 'aria-hidden="true" tabindex="-1"' : 'tabindex="0" role="button"';
        return `
          <div class="glass-card cert-card" data-category="${c.category || ''}" data-idx="${idx}"
               data-img="${imgSrc}" data-caption="${caption}"
               aria-label="View certificate: ${c.title}" ${aria}>
            <div class="cert-thumb-holder">
              <img src="${imgSrc}" alt="${c.title}" class="cert-thumb-img" loading="lazy" onerror="this.style.opacity='0.6'" />
            </div>
            <div class="cert-caption-box">
              <h4 class="cert-title">${c.title}</h4>
              <p class="cert-meta-line">${c.detail ? c.detail + ' \u00b7 ' : ''}${c.role}</p>
              <span class="cert-date-chip">${c.date || 'Date: TBA'}</span>
            </div>
          </div>`;
      }).join('');
    }

    /* ============================================================
       3. PROJECTS (S10_LOOP — Scene 10 Summit Loop after flight)
       ============================================================ */
    _projects() {
      const pData = this.data.projects || {};
      const featured = pData.featured || {
        number: '21',
        name: 'SAATHI',
        title: 'SAATHI — BIS & AIS Statutory RAG Engine',
        sihCode: 'SIH26107',
        tagline: 'AI assistant for BIS regulations and industrial compliance',
        desc: 'Specialized intelligence system for Bureau of Indian Standards (BIS) regulations, built for the Smart India Hackathon 2026. Automates regulatory search, real-time standards comparison, and multi-clause compliance audits.',
        status: 'LIVE-FULL',
        statusBadge: 'Round 2 Qualified',
        statusClass: 'chip-breathing',
        live: 'https://avishjain9.github.io/21-bis-compliance-engine/',
        liveLabel: 'Live Demo',
        github: 'https://github.com/AVISHJAIN9/21-bis-compliance-engine',
        doc: 'docs/saathi.pdf'
      };
      const items = pData.items || [];

      const statusMap = {
        'LIVE-FULL': { cls: 'chip-live', label: 'Live Full-Stack' },
        'FRONTEND-ONLY': { cls: 'chip-frontend', label: 'Frontend App' },
        'BACKEND-ONLY': { cls: 'chip-backend', label: 'Backend API' },
        'UNDER-CONSTRUCTION': { cls: 'chip-under-construction', label: 'Under Construction' }
      };

      return `
        <div class="panel-frost-sparkles" aria-hidden="true">
          <span class="sparkle s1">&#x2726;</span><span class="sparkle s2">&#x2726;</span>
          <span class="sparkle s3">&#x2726;</span><span class="sparkle s4">&#x2726;</span>
        </div>
        <header class="glass-panel-header sticky-header">
          <div>
            <h2 class="panel-main-title">Projects Showcase</h2>
            <p class="panel-subtitle-text">Production systems, microservices &amp; full-stack applications built from first principles.</p>
          </div>
          <button class="icon-btn panel-close-btn" data-close-panel aria-label="Close Projects">&#x2715;</button>
        </header>
        <div class="glass-panel-content">

          <!-- Featured Project Card: SAATHI -->
          <div class="glass-card featured-project-card frost-edge-border">
            <div class="featured-header">
              <div class="featured-badge-group">
                <span class="status-chip chip-breathing"><span class="breathing-dot"></span>Round 2 Qualified</span>
                <span class="tag-chip lang-chip">${featured.sihCode || featured.sihId || 'SIH26107'}</span>
              </div>
              <span class="featured-index">FEATURED SYSTEM #${featured.number || '21'}</span>
            </div>
            <h3 class="featured-title">${featured.name}</h3>
            <p class="featured-tagline">${featured.tagline || featured.subtitle || ''}</p>
            <p class="featured-desc">${featured.description || featured.desc || ''}</p>
            <div class="card-button-row">
              <button class="glass-pill-btn doc-pdf-btn" data-pdf="${featured.doc || featured.pdf || 'docs/saathi.pdf'}">Documentation (PDF)</button>
              ${this._glassLink(featured.live, featured.liveLabel || 'Live Demo', true)}
              ${this._glassLink(featured.github, 'GitHub \u2197')}
            </div>
          </div>

          <!-- Search & Filter Controls -->
          <div class="project-controls">
            <div class="search-input-box">
              <span class="search-icon" aria-hidden="true">&#x1F50D;</span>
              <input type="text" id="project-search-input" class="glass-input" placeholder="Search systems engines..." aria-label="Search projects"/>
            </div>
            <div class="filter-chips-row" role="tablist" aria-label="Filter projects">
              <button class="glass-chip active" data-filter="all">All (${items.length})</button>
              <button class="glass-chip" data-filter="LIVE-FULL">Full-Stack (${items.filter(i => i.status === 'LIVE-FULL').length})</button>
              <button class="glass-chip" data-filter="FRONTEND-ONLY">Frontend (${items.filter(i => i.status === 'FRONTEND-ONLY').length})</button>
              <button class="glass-chip" data-filter="BACKEND-ONLY">Backend API (${items.filter(i => i.status === 'BACKEND-ONLY').length})</button>
            </div>
          </div>

          <!-- 39 Projects Grid -->
          <div class="projects-grid" id="projects-grid">
            ${items.map(item => {
              const meta = statusMap[item.status] || { cls: 'chip-in-progress', label: item.status || 'Active' };
              const liveBtnLabel = (item.liveLabel || (item.status === 'BACKEND-ONLY' ? 'API / Docs' : 'Live Demo')) + ' \u2197';
              return `
                <div class="glass-card project-card" data-status="${item.status || ''}" data-title="${(item.name || item.title || '').toLowerCase()}">
                  <div class="card-top-row">
                    <span class="project-index-num">#${item.number}</span>
                    <span class="status-chip ${meta.cls}"><span class="chip-dot"></span>${meta.label}</span>
                  </div>
                  <h4 class="project-repo-name">${item.title || item.name}</h4>
                  <p class="project-one-liner">${item.desc || item.description || ''}</p>
                  <div class="project-tech-row">
                    <span class="tag-chip lang-chip">${item.language || 'Systems'}</span>
                    <span class="tag-chip host-chip">${item.host || 'Render'}</span>
                  </div>
                  <div class="card-button-row">
                    ${item.doc ? (item.doc.endsWith('.pdf') ? `<button class="glass-pill-btn doc-pdf-btn" data-pdf="${item.doc}">Doc (PDF)</button>` : `<a class="glass-pill-btn" href="${item.doc}" target="_blank" rel="noopener noreferrer">Docs \u2197</a>`) : ''}
                    ${this._glassLink(item.live, liveBtnLabel, true)}
                    ${this._glassLink(item.github, 'GitHub \u2197')}
                  </div>
                </div>`;
            }).join('')}
          </div>

        </div>`;
    }

    _glassLink(url, label, isHighlight = false) {
      if (url && url !== '#' && !url.includes('undefined')) {
        const cls = isHighlight ? 'glass-pill-btn highlight-btn' : 'glass-pill-btn';
        return `<a class="${cls}" href="${url}" target="_blank" rel="noopener noreferrer">${label}</a>`;
      }
      return `<span class="glass-pill-btn disabled-upcoming" title="Link coming soon" aria-disabled="true">Upcoming</span>`;
    }

    /* ============================================================
       4. SKILLS (S12_INTERIOR — Inside Cabin at Craftsman's Table)
       ============================================================ */
    _skills() {
      const groups = this.data.skills || [];
      const lvlCls = { Fluent: 'skill-fluent', Proficient: 'skill-proficient', Learning: 'skill-learning' };
      return `
        <div class="panel-frost-sparkles" aria-hidden="true">
          <span class="sparkle s1">&#x2726;</span><span class="sparkle s2">&#x2726;</span>
          <span class="sparkle s3">&#x2726;</span><span class="sparkle s4">&#x2726;</span>
        </div>
        <header class="glass-panel-header sticky-header">
          <div>
            <h2 class="panel-main-title">Skills &amp; Architecture</h2>
            <p class="panel-subtitle-text">Drafted from first principles upon the craftsman's table</p>
          </div>
          <button class="icon-btn panel-close-btn" data-close-panel aria-label="Close Skills">&#x2715;</button>
        </header>
        <div class="glass-panel-content">
          <div class="skills-groups">
            ${groups.map(g => `
              <div class="skills-group">
                <h3 class="skills-group-title">${g.category}</h3>
                <div class="skills-chips-row">
                  ${(g.skills || []).map(sk => `
                    <div class="glass-card skill-card">
                      <div class="shine-sweep" aria-hidden="true"></div>
                      <span class="skill-name">${sk.name}</span>
                      <span class="skill-level-chip ${lvlCls[sk.level] || ''}">${sk.level}</span>
                    </div>`).join('')}
                </div>
              </div>`).join('')}
          </div>

          <div class="skills-workbench-footer">
            <span class="workbench-hint">The blueprints are drawn. Look out the frosted window toward the winter cosmos.</span>
            <button id="btn-skills-to-finale" class="glass-pill-btn highlight-btn" aria-label="Gaze into Winter Aurora">
              Gaze into Celestial Aurora &rarr;
            </button>
          </div>
        </div>`;
    }

    /* ============================================================
       5. CELESTIAL AURORA (S13_FINALE — Startup above Contact + Aurora Mention above Clubs)
       ============================================================ */
    _finale() {
      const s = this.data.startup || {};
      const p = this.data.profile || {};
      const contact = p.contact || {
        email: 'avishjain412@gmail.com',
        linkedin: 'https://www.linkedin.com/in/avish-jain-956962362',
        github: 'https://github.com/AVISHJAIN9'
      };
      const quote = p.quote || 'Simplicity is prerequisite for reliability.';

      return `
        <!-- Left Column: Startup Card directly above Contact Card -->
        <div class="finale-left-column">
          <!-- Startup Venture Card -->
          <div class="finale-startup-card glass-card frost-edge-border" id="finale-startup-card">
            <div class="startup-meta-top">
              <span class="status-chip chip-in-progress"><span class="chip-dot"></span>Active Venture</span>
              <span class="tag-chip">Biotechnology &amp; Simulation</span>
            </div>
            <h3 class="startup-brand">${s.name || 'All Tech'}</h3>
            <p class="startup-founder">Founder: <strong>${s.founder || 'Avish Jain'}</strong></p>
            <p class="startup-org">Operating under: <strong>${s.operatingUnder || 'BIG Foundation Corporation'}</strong></p>
            <p class="startup-domain">Domain: <strong>${s.domain || 'in-silico drug testing'}</strong></p>
            <p class="startup-desc">${s.description || 'Pioneering computational pharmaceutical infrastructure and drug testing simulations in-silico.'}</p>
            <div class="startup-doc-row">
              <span class="glass-pill-btn disabled-upcoming" title="Link coming soon" aria-disabled="true">Detailed document: Upcoming</span>
            </div>
          </div>

          <!-- Contact Details Card -->
          <div class="finale-contact-block glass-card" id="finale-contact-block">
            <h3 class="finale-contact-heading">Contact</h3>
            <div class="contact-pill-item">
              <span class="contact-label">Email</span>
              <a href="mailto:${contact.email}" class="contact-val">${contact.email}</a>
              <button class="glass-pill-btn copy-email-btn" data-email="${contact.email}" aria-label="Copy email">Copy</button>
            </div>
            <div class="contact-pill-item">
              <span class="contact-label">LinkedIn</span>
              <span class="contact-val">Avish Jain</span>
              <a href="${contact.linkedin}" class="glass-pill-btn" target="_blank" rel="noopener noreferrer">Connect &#x2197;</a>
            </div>
            <div class="contact-pill-item">
              <span class="contact-label">GitHub</span>
              <span class="contact-val">AVISHJAIN9</span>
              <a href="${contact.github}" class="glass-pill-btn" target="_blank" rel="noopener noreferrer">Follow &#x2197;</a>
            </div>
            ${quote ? `<p class="finale-quote">&ldquo;${quote}&rdquo;</p>` : ''}
          </div>
        </div>

        <!-- Right Column: Aurora Mention Box above Clubs & Stacked Buttons -->
        <div class="finale-right-column">
          <!-- Poetic Aurora Mention Card -->
          <div class="aurora-mention-box glass-card frost-edge-border">
            <span class="aurora-badge">✨ CELESTIAL AURORA</span>
            <h4 class="aurora-title">The Northern Horizon</h4>
            <p class="aurora-desc">Under glowing constellations and polar lights, the journey reaches its zenith. Explore the club communities or inspect forthcoming writings.</p>
          </div>

          <!-- 4 Stacked Buttons -->
          <div class="finale-buttons-stack" role="navigation" aria-label="Explore sections">
            <button id="btn-finale-clubs" class="finale-side-btn highlight-btn" aria-label="Clubs Showcase">
              <span class="finale-btn-icon" aria-hidden="true">&#x1F3AF;</span>Clubs
            </button>
            <button id="btn-finale-future" class="finale-side-btn highlight-btn" aria-label="Future Projects / Coming Soon">
              <span class="finale-btn-icon" aria-hidden="true">&#x1F6A7;</span>Future Projects
            </button>
            <button id="btn-finale-blog" class="finale-side-btn" aria-label="Blog">
              <span class="finale-btn-icon" aria-hidden="true">&#x270D;</span>Blog
            </button>
            <button id="btn-finale-upcoming" class="finale-side-btn" aria-label="Upcoming Events">
              <span class="finale-btn-icon" aria-hidden="true">&#x1F4C5;</span>Upcoming Events
            </button>
          </div>
        </div>`;
    }

    /* ============================================================
       SUB-PANEL: BLOG
       ============================================================ */
    _blog() {
      const posts = this.data.blog || [];
      return `
        <div class="panel-frost-sparkles" aria-hidden="true">
          <span class="sparkle s1">&#x2726;</span><span class="sparkle s2">&#x2726;</span><span class="sparkle s3">&#x2726;</span>
        </div>
        <header class="glass-panel-header sticky-header">
          <div>
            <h2 class="panel-main-title">Blog</h2>
            <p class="panel-subtitle-text">Written posts and videos &mdash; all in one place</p>
          </div>
          <button class="icon-btn panel-close-btn sub-panel-close" data-sub-panel-close aria-label="Back">&larr; Back</button>
        </header>
        <div class="glass-panel-content">
          ${posts.length === 0 ? `
            <div class="glass-card empty-state-card">
              <span class="empty-icon" aria-hidden="true">&#x270D;</span>
              <h3 class="empty-title">Posts and videos coming soon</h3>
              <p class="empty-desc">Engineering deep-dives and walkthroughs will appear here automatically as blog.json is updated.</p>
              <span class="glass-pill-btn disabled-upcoming" title="Link coming soon" aria-disabled="true">Upcoming</span>
            </div>` : `
            <div class="blog-grid">
              ${posts.map(post => `
                <article class="glass-card blog-card">
                  ${post.cover ? `<img src="${post.cover}" alt="${post.title}" class="blog-cover" loading="lazy" />` : ''}
                  <div class="blog-card-type-chip">${post.type === 'video' ? '&#x25B6; Video' : '&#x270D; Post'}</div>
                  <h4 class="blog-card-title">${post.title}</h4>
                  <span class="blog-date">${post.date || ''}</span>
                  <p class="blog-summary">${post.summary || ''}</p>
                  <div class="card-button-row">
                    ${post.type === 'video'
                      ? `<button class="glass-pill-btn" data-video-src="${post.videoSrc || post.url}">&#x25B6; Play</button>`
                      : `<a class="glass-pill-btn" href="${post.url}" target="_blank" rel="noopener noreferrer">Read &#x2197;</a>`}
                  </div>
                </article>`).join('')}
            </div>`}
        </div>`;
    }

    /* ============================================================
       SUB-PANEL: UPCOMING EVENTS
       ============================================================ */
    _upcoming() {
      const items = this.data.upcoming || [];
      const badgeCls = {
        'Registered': 'chip-registered',
        'Planned':    'chip-planned',
        'Breathing':  'chip-breathing',
        'Upcoming':   'chip-upcoming'
      };
      return `
        <div class="panel-frost-sparkles" aria-hidden="true">
          <span class="sparkle s1">&#x2726;</span><span class="sparkle s2">&#x2726;</span><span class="sparkle s3">&#x2726;</span>
        </div>
        <header class="glass-panel-header sticky-header">
          <div>
            <h2 class="panel-main-title">Upcoming Events</h2>
            <p class="panel-subtitle-text">What's next &mdash; hackathons, symposiums &amp; workshops</p>
          </div>
          <button class="icon-btn panel-close-btn sub-panel-close" data-sub-panel-close aria-label="Back">&larr; Back</button>
        </header>
        <div class="glass-panel-content">
          <div class="upcoming-grid">
            ${items.map(item => {
              const cls = badgeCls[item.badge || item.status] || 'chip-upcoming';
              return `
                <div class="glass-card upcoming-card">
                  <div class="upcoming-top">
                    <span class="status-chip ${cls}">
                      ${item.badge === 'Registered' ? '<span class="glow-dot"></span>' : ''}
                      ${item.status}
                    </span>
                  </div>
                  <h4 class="upcoming-title">${item.title}</h4>
                  <p class="upcoming-date">${item.date || 'Date: TBA'}</p>
                </div>`;
            }).join('')}
          </div>
        </div>`;
    }

    /* ============================================================
       SUB-PANEL: FUTURE PROJECTS / COMING SOON
       ============================================================ */
    _futureProjects() {
      const pData = this.data.projects || {};
      const futures = pData.futureProjects || [];

      return `
        <div class="panel-frost-sparkles" aria-hidden="true">
          <span class="sparkle s1">&#x2726;</span><span class="sparkle s2">&#x2726;</span><span class="sparkle s3">&#x2726;</span><span class="sparkle s4">&#x2726;</span>
        </div>
        <header class="glass-panel-header sticky-header">
          <div>
            <div class="future-header-badge-row">
              <span class="status-chip chip-under-construction"><span class="chip-dot"></span>Under Construction</span>
              <span class="future-count-chip">${futures.length} Systems Engines</span>
            </div>
            <h2 class="panel-main-title">Future Projects / Coming Soon</h2>
            <p class="panel-subtitle-text">Distributed kernels, low-latency protocols, and systems architectures currently under construction from first principles.</p>
          </div>
          <div class="future-header-actions">
            <button class="icon-btn panel-close-btn sub-panel-close" data-sub-panel-close aria-label="Back">&larr; Back</button>
            <button class="icon-btn panel-close-btn" data-close-panel aria-label="Close Future Projects">&#x2715;</button>
          </div>
        </header>
        <div class="glass-panel-content">
          <div class="future-projects-grid">
            ${futures.map(p => `
              <div class="glass-card future-card frost-edge-border" data-num="${p.number}">
                <div class="card-top-row">
                  <span class="project-index-num">#${p.number}</span>
                  <span class="status-chip chip-under-construction"><span class="chip-dot"></span>Under Construction</span>
                </div>
                <h3 class="future-card-title">${p.number} ${p.name}</h3>
                <div class="future-type-chip">${p.type}</div>
                <p class="future-card-desc">${p.desc || ''}</p>
                <div class="card-button-row">
                  ${p.github ? `<a class="glass-pill-btn" href="${p.github}" target="_blank" rel="noopener noreferrer">GitHub Repo \u2197</a>` : ''}
                  <span class="glass-pill-btn disabled-upcoming" title="Engine under construction" aria-disabled="true">In R&amp;D</span>
                </div>
              </div>`).join('')}
          </div>

          <!-- Bottom Call to Action Card -->
          <div class="glass-card future-cta-card frost-edge-border">
            <div class="cta-sparkle" aria-hidden="true">&#x2726;</div>
            <h3 class="cta-title">Collaborate on Systems Architecture</h3>
            <p class="cta-text">Interested in any of these or want to collaborate? Mail me at <a href="mailto:avishjain412@gmail.com?subject=Interested%20in%20your%20Future%20Projects" class="cta-email-inline">avishjain412@gmail.com</a></p>
            <div class="cta-action-row">
              <a class="glass-pill-btn highlight-btn cta-mailto-btn" href="mailto:avishjain412@gmail.com?subject=Interested%20in%20your%20Future%20Projects">
                &#x2709; Mail me at avishjain412@gmail.com
              </a>
              <button class="glass-pill-btn copy-email-btn" data-email="avishjain412@gmail.com">Copy Email</button>
            </div>
          </div>
        </div>`;
    }

    /* ============================================================
       STATE DISPATCHER
       ============================================================ */
    onStateChange(stateName) {
      console.log(`[GLASS] onStateChange -> ${stateName}`);

      // Hide all panels
      document.querySelectorAll('.glass-page-layer').forEach(p => {
        p.style.display = 'none';
        p.classList.remove('panel-visible');
      });
      document.body.classList.remove('glass-panel-active');
      this.finaleSubPanel = null;
      this.activePanelId = null;

      // Exact Mapping:
      // Chapter 4: S7_REINDEER  -> panel-intro (About Me + Contact)
      // Chapter 5: S9_ICE_CAVE  -> panel-achievements (Achievements + Continue button)
      // Chapter 6: S8_HOLD      -> none (free exploration)
      // Scene 10:  S10_LOOP     -> panel-projects (Projects)
      // Chapter 7: S_HOUSE      -> none (free exploration, door to enter)
      // Scene 12:  S12_INTERIOR -> panel-skills (Skills at table)
      // Scene 13:  S13_FINALE   -> panel-finale (Startup + Contact on left; Aurora + Buttons on right)
      const map = {
        S7_REINDEER:  'panel-intro',
        S9_ICE_CAVE:  'panel-achievements',
        S10_LOOP:     'panel-projects',
        S12_INTERIOR: 'panel-skills',
        S13_FINALE:   'panel-finale',
      };

      const id = map[stateName];
      if (!id) return;

      const el = document.getElementById(id);
      if (!el) return;

      this.activePanelId = id;
      document.body.classList.add('glass-panel-active');

      el.style.display = 'flex';
      setTimeout(() => {
        el.classList.add('panel-visible');
        if (id === 'panel-achievements' || id === 'panel-intro') {
          this._countUp();
          this._restartScroll();
        }
      }, 120);
    }

    /* ============================================================
       GLOBAL INTERACTIONS & EVENT DELEGATION
       ============================================================ */
    setupGlobalInteractions() {
      document.addEventListener('click', e => {
        // Close panel button
        if (e.target.closest('[data-close-panel]')) {
          const panel = e.target.closest('.glass-panel');
          if (panel) {
            panel.classList.remove('panel-visible');
            setTimeout(() => {
              panel.style.display = 'none';
              document.body.classList.remove('glass-panel-active');
            }, 300);
          }
        }

        // Sub-panel back button
        if (e.target.closest('[data-sub-panel-close]')) {
          ['panel-blog', 'panel-upcoming', 'panel-future-projects'].forEach(sid => {
            const sp = document.getElementById(sid);
            if (sp) {
              sp.classList.remove('panel-visible');
              setTimeout(() => (sp.style.display = 'none'), 300);
            }
          });
          this.finaleSubPanel = null;
          const f = document.getElementById('panel-finale');
          if (f) {
            f.style.display = 'flex';
            setTimeout(() => f.classList.add('panel-visible'), 20);
          }
        }

        // Intro: "Venture into the Glacial Cave" button
        if (e.target.closest('#btn-intro-continue') && window.PortfolioApp) {
          this._dismissAllPanels();
          window.PortfolioApp.runTransitionSequence(['reindeer-run-to-cave'], 'S9_ICE_CAVE');
        }

        // Cave: "Continue to Alpine Summit" continuation button
        if (e.target.closest('#btn-cave-continue') && window.PortfolioApp) {
          this._dismissAllPanels();
          window.PortfolioApp.runTransitionSequence(['cave-exit-connector', 'reindeer-cliff-run'], 'S8_HOLD');
        }

        // Skills: "Gaze into Celestial Aurora" advance button
        if (e.target.closest('#btn-skills-to-finale') && window.PortfolioApp) {
          this._dismissAllPanels();
          const s12End = document.getElementById('s12-end-frame-img');
          if (s12End) s12End.style.display = 'none';
          window.PortfolioApp.runTransitionSequence(['blueprint-transition', 'celestial-finale'], 'S13_FINALE');
        }

        // Finale: Clubs
        if (e.target.closest('#btn-finale-clubs') && window.PortfolioApp) {
          window.PortfolioApp.transitionToScene14();
        }

        // Finale: Future Projects
        if (e.target.closest('#btn-finale-future')) {
          this._openSubPanel('panel-future-projects');
        }

        // Finale: Blog
        if (e.target.closest('#btn-finale-blog')) {
          this._openSubPanel('panel-blog');
        }

        // Finale: Upcoming Events
        if (e.target.closest('#btn-finale-upcoming')) {
          this._openSubPanel('panel-upcoming');
        }

        // Project filter chips
        const fchip = e.target.closest('.filter-chips-row .glass-chip');
        if (fchip) {
          fchip.closest('.filter-chips-row').querySelectorAll('.glass-chip').forEach(c => c.classList.remove('active'));
          fchip.classList.add('active');
          this.currentProjectFilter = fchip.getAttribute('data-filter');
          this._filterProjects();
        }

        // Cert filter chips
        const cchip = e.target.closest('.cert-filter-chips .glass-chip');
        if (cchip) {
          cchip.closest('.cert-filter-chips').querySelectorAll('.glass-chip').forEach(c => c.classList.remove('active'));
          cchip.classList.add('active');
          this._filterCerts(cchip.getAttribute('data-cert-filter'));
        }

        // Copy email
        const copyBtn = e.target.closest('.copy-email-btn');
        if (copyBtn) {
          const email = copyBtn.getAttribute('data-email') || 'avishjain412@gmail.com';
          navigator.clipboard.writeText(email)
            .then(() => this._toast('Email copied: ' + email))
            .catch(() => this._toast(email));
        }

        // Cert card click -> lightbox
        const certCard = e.target.closest('.cert-card');
        if (certCard && !e.target.closest('a') && !e.target.closest('button')) {
          const src = certCard.getAttribute('data-img');
          const cap = certCard.getAttribute('data-caption');
          if (src) this._openLightbox(src, cap);
        }

        // PDF Documentation Modal Trigger
        const pdfBtn = e.target.closest('.doc-pdf-btn');
        if (pdfBtn && !pdfBtn.classList.contains('disabled-upcoming')) {
          const pdfPath = pdfBtn.getAttribute('data-pdf');
          if (pdfPath) this._openPDF(pdfPath, 'Project Documentation');
        }
      });

      // Search projects
      document.addEventListener('input', e => {
        if (e.target && e.target.id === 'project-search-input') {
          this.projectSearchQuery = e.target.value.toLowerCase().trim();
          this._filterProjects();
        }
      });

      // Keyboard navigation for cards
      document.addEventListener('keydown', e => {
        if ((e.key === 'Enter' || e.key === ' ') && e.target.closest('.cert-card[role="button"]')) {
          e.preventDefault();
          e.target.closest('.cert-card').click();
        }
      });
    }

    _dismissAllPanels() {
      // Instantly hide every glass panel before a scene transition
      document.querySelectorAll('.glass-page-layer').forEach(p => {
        p.classList.remove('panel-visible');
        p.style.display = 'none';
      });
      document.body.classList.remove('glass-panel-active');
      this.finaleSubPanel = null;
      this.activePanelId = null;
    }

    _openSubPanel(id) {
      const f = document.getElementById('panel-finale');
      if (f) {
        f.classList.remove('panel-visible');
        setTimeout(() => (f.style.display = 'none'), 250);
      }
      const sp = document.getElementById(id);
      if (!sp) return;
      this.finaleSubPanel = id;
      document.body.classList.add('glass-panel-active');
      sp.style.display = 'flex';
      setTimeout(() => sp.classList.add('panel-visible'), 100);
    }

    _filterProjects() {
      document.querySelectorAll('#projects-grid .project-card').forEach(c => {
        const title = (c.getAttribute('data-title') || '').toLowerCase();
        const status = c.getAttribute('data-status') || '';
        const matchSearch = !this.projectSearchQuery || title.includes(this.projectSearchQuery);
        const matchStatus = this.currentProjectFilter === 'all' || status === this.currentProjectFilter;
        c.style.display = (matchSearch && matchStatus) ? 'flex' : 'none';
      });
    }

    _filterCerts(cat) {
      document.querySelectorAll('#cert-marquee-track .cert-card').forEach(c => {
        const cCat = c.getAttribute('data-category');
        c.style.display = (cat === 'all' || cCat === cat) ? 'flex' : 'none';
      });
    }

    _toast(msg) {
      const t = document.getElementById('glass-toast');
      if (!t) return;
      t.textContent = msg;
      t.classList.add('show');
      setTimeout(() => t.classList.remove('show'), 2500);
    }

    /* ============================================================
       3D TILT & SPOTLIGHT
       ============================================================ */
    setupA3TiltAndSpotlight() {
      if (window.matchMedia('(prefers-reduced-motion: reduce)').matches || window.innerWidth < 900) return;
      document.addEventListener('mousemove', e => {
        const panel = document.querySelector('.glass-panel.panel-visible');
        if (panel) {
          const r = panel.getBoundingClientRect();
          panel.style.setProperty('--spotlight-x', `${e.clientX - r.left}px`);
          panel.style.setProperty('--spotlight-y', `${e.clientY - r.top}px`);
        }
        const card = e.target.closest('.glass-card');
        if (!card) return;
        const r = card.getBoundingClientRect();
        const cx = r.width / 2, cy = r.height / 2;
        const rx = ((e.clientY - r.top - cy) / cy) * -4;
        const ry = ((e.clientX - r.left - cx) / cx) * 4;
        card.style.transform = `perspective(900px) rotateX(${rx.toFixed(2)}deg) rotateY(${ry.toFixed(2)}deg) translateY(-4px) scale(1.015)`;
      });
      document.addEventListener('mouseout', e => {
        const card = e.target.closest('.glass-card');
        if (card && (!e.relatedTarget || !card.contains(e.relatedTarget))) {
          card.style.transform = '';
        }
      });
    }

    /* ============================================================
       COUNT-UP ANIMATION
       ============================================================ */
    _countUp() {
      if (this.statsAnimated) return;
      this.statsAnimated = true;
      document.querySelectorAll('.stat-number[data-count-to]').forEach(el => {
        const target = parseInt(el.getAttribute('data-count-to'), 10) || 0;
        const t0 = performance.now();
        const tick = now => {
          const t = Math.min((now - t0) / 1200, 1);
          el.textContent = Math.floor(target * (1 - Math.pow(1 - t, 3)));
          if (t < 1) requestAnimationFrame(tick);
          else el.textContent = target;
        };
        requestAnimationFrame(tick);
      });
    }

    /* ============================================================
       CERTIFICATE AUTO-SCROLL LOOP
       ============================================================ */
    setupCertificatesAutoScroll() {
      if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
      const track = document.getElementById('cert-marquee-track');
      const wrap  = document.getElementById('cert-marquee-container');
      if (!track || !wrap) return;

      let pos = 0;
      const speed = 0.55;

      const pause  = () => { this.autoScrollPaused = true;  };
      const resume = () => { this.autoScrollPaused = false; };

      wrap.addEventListener('mouseenter', pause);
      wrap.addEventListener('mouseleave', resume);
      wrap.addEventListener('touchstart', pause,  { passive: true });
      wrap.addEventListener('touchend',   resume, { passive: true });
      wrap.addEventListener('focusin',    pause);
      wrap.addEventListener('focusout',   resume);

      const loop = () => {
        if (!this.autoScrollPaused && track.parentElement) {
          pos += speed;
          const half = track.scrollHeight / 2;
          if (half > 0 && pos >= half) pos = 0;
          track.style.transform = `translate3d(0, -${pos}px, 0)`;
        }
        this.autoScrollAnimId = requestAnimationFrame(loop);
      };
      this.autoScrollAnimId = requestAnimationFrame(loop);
    }

    _restartScroll() {
      if (this.autoScrollAnimId) cancelAnimationFrame(this.autoScrollAnimId);
      const t = document.getElementById('cert-marquee-track');
      if (t) t.style.transform = 'translate3d(0,0,0)';
      this.autoScrollPaused = false;
      this.setupCertificatesAutoScroll();
    }

    /* ============================================================
       LIGHTBOX
       ============================================================ */
    setupLightbox() {
      const m = document.getElementById('cert-lightbox');
      if (!m) return;
      m.querySelector('.lightbox-close').addEventListener('click',    () => this._closeLightbox());
      m.querySelector('.lightbox-backdrop').addEventListener('click', () => this._closeLightbox());
      m.querySelector('.lightbox-prev').addEventListener('click',     () => this._navLightbox(-1));
      m.querySelector('.lightbox-next').addEventListener('click',     () => this._navLightbox(1));
      document.addEventListener('keydown', e => {
        if (m.style.display !== 'none') {
          if (e.key === 'Escape')     this._closeLightbox();
          if (e.key === 'ArrowLeft')  this._navLightbox(-1);
          if (e.key === 'ArrowRight') this._navLightbox(1);
        }
      });
    }

    _openLightbox(src, cap) {
      const m = document.getElementById('cert-lightbox');
      const img = document.getElementById('lightbox-img');
      if (!m || !img) return;
      const certs = (this.data.certificates || []).filter(c => c.image);
      this.activeCertificates = certs;
      this.lightboxIndex = Math.max(0, certs.findIndex(c => c.image === src));
      img.src = src;
      const capEl = document.getElementById('lightbox-caption');
      const dl    = document.getElementById('lightbox-download');
      if (capEl) capEl.textContent = cap || '';
      if (dl)    dl.href = src;
      m.style.display = 'flex';
      requestAnimationFrame(() => m.classList.add('open'));
      m.setAttribute('aria-hidden', 'false');
    }

    _navLightbox(dir) {
      if (!this.activeCertificates.length) return;
      this.lightboxIndex = (this.lightboxIndex + dir + this.activeCertificates.length) % this.activeCertificates.length;
      const c = this.activeCertificates[this.lightboxIndex];
      const img = document.getElementById('lightbox-img');
      const cap = document.getElementById('lightbox-caption');
      const dl  = document.getElementById('lightbox-download');
      if (img) img.src = c.image;
      if (cap) cap.textContent = [c.title, c.detail, c.role].filter(Boolean).join(' \u00b7 ');
      if (dl)  dl.href = c.image;
    }

    _closeLightbox() {
      const m = document.getElementById('cert-lightbox');
      if (!m) return;
      m.classList.remove('open');
      setTimeout(() => {
        m.style.display = 'none';
        m.setAttribute('aria-hidden', 'true');
      }, 250);
    }

    /* ============================================================
       PDF VIEWER
       ============================================================ */
    setupPDFViewer() {
      const m = document.getElementById('pdf-modal');
      if (!m) return;
      m.querySelector('.pdf-modal-backdrop').addEventListener('click', () => this._closePDF());
      document.getElementById('pdf-modal-close').addEventListener('click', () => this._closePDF());
      document.addEventListener('keydown', e => {
        if (m.style.display !== 'none' && e.key === 'Escape') this._closePDF();
      });
    }

    _openPDF(url, title = 'Document') {
      const m = document.getElementById('pdf-modal');
      const frame = document.getElementById('pdf-modal-iframe');
      const t = document.getElementById('pdf-modal-title');
      const dl = document.getElementById('pdf-modal-download');
      if (!m || !frame) return;
      frame.src = url;
      if (t) t.textContent = title;
      if (dl) dl.href = url;
      m.style.display = 'flex';
      requestAnimationFrame(() => m.classList.add('open'));
      m.setAttribute('aria-hidden', 'false');
    }

    _closePDF() {
      const m = document.getElementById('pdf-modal');
      const frame = document.getElementById('pdf-modal-iframe');
      if (!m) return;
      m.classList.remove('open');
      setTimeout(() => {
        m.style.display = 'none';
        if (frame) frame.src = '';
        m.setAttribute('aria-hidden', 'true');
      }, 250);
    }
  }

  // Export singleton to window
  window.GlassPortfolio = new GlassPortfolio();
})();
