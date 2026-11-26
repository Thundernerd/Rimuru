// CsRinRu Desktop - Main Application Script

/**
 * Main Application Module
 * Handles the game feed UI and interaction with the backend
 */

class CsRinRuApp {
    constructor() {
        this.currentGames = [];
        this.currentGameIndex = 0;
        this.expandedGames = new Set();
        
        this.init();
    }
    
    /**
     * Initialize the application
     */
    init() {
        this.cacheElements();
        this.bindEvents();
        this.loadGames();
    }
    
    /**
     * Cache DOM elements for performance
     */
    cacheElements() {
        this.gameListEl = document.getElementById('game-list');
        this.refreshBtn = document.getElementById('refresh-btn');
        this.prevPageBtn = document.getElementById('prev-page');
        this.nextPageBtn = document.getElementById('next-page');
        this.pageInfoEl = document.getElementById('page-info');
        this.detailPanelEl = document.getElementById('game-detail-panel');
        this.detailContentEl = document.getElementById('panel-content');
        this.closeDetailBtn = document.getElementById('close-detail');
    }
    
    /**
     * Bind event listeners
     */
    bindEvents() {
        this.refreshBtn.addEventListener('click', () => this.loadGames());
        this.prevPageBtn.addEventListener('click', () => this.navigateToPage(this.currentGameIndex - 1));
        this.nextPageBtn.addEventListener('click', () => this.navigateToPage(this.currentGameIndex + 1));
        this.closeDetailBtn.addEventListener('click', () => this.closeDetailPanel());
    }
    
    /**
     * Load games from the forum via Tauri invoke
     */
    async loadGames() {
        this.showLoadingState();
        
        try {
            // Invoke the backend to fetch games
            const games = await window.__TAURI__.invoke('fetch_game_feed', {
                page: this.currentGameIndex
            });
            
            this.currentGames = games;
            this.renderGameList();
            this.updatePagination();
        } catch (error) {
            console.error('Error loading games:', error);
            this.showError('Failed to load games. Please try again.');
        }
    }
    
    /**
     * Render the game list
     */
    renderGameList() {
        if (this.currentGames.length === 0) {
            this.renderEmptyState();
            return;
        }
        
        const gamesHtml = this.currentGames.map((game, index) => {
            const isExpanded = this.expandedGames.has(game.id);
            return `
                <div class="game-item ${isExpanded ? 'game-item-expanded' : ''}" 
                     data-game-id="${game.id}" 
                     data-index="${index}">
                    <img src="${game.image || '/assets/default-game.png'}" 
                         alt="${game.title}" 
                         class="game-item-image"
                         onerror="this.src='/assets/default-game.png'">
                    <div class="game-item-content">
                        <h4 class="game-item-title">${this.escapeHtml(game.title)}</h4>
                        <div class="game-item-meta">
                            <span class="game-author">By ${this.escapeHtml(game.author)}</span>
                            <span class="game-date">${game.postDate}</span>
                        </div>
                        ${isExpanded ? this.renderExpandedContent(game) : ''}
                    </div>
                </div>
            `;
        }).join('');
        
        this.gameListEl.innerHTML = gamesHtml;
        this.bindGameEvents();
    }
    
    /**
     * Render expanded content for a game
     */
    renderExpandedContent(game) {
        return `
            <div class="game-item-expansion">
                <div class="posts-container">
                    ${game.posts && game.posts.length > 0 ? game.posts.map(post => `
                        <div class="game-post">
                            <div class="game-post-header">
                                <span>${this.escapeHtml(post.author)}</span>
                                <span>${post.date}</span>
                            </div>
                            <p>${this.escapeHtml(post.content.substring(0, 200))}${post.content.length > 200 ? '...' : ''}</p>
                            ${post.isReleasePost ? '<span class="release-badge">Release</span>' : ''}
                            ${post.downloadLinks && post.downloadLinks.length > 0 ? `
                                <div class="download-links">
                                    ${post.downloadLinks.map(link => `
                                        <a href="${link}" class="download-link" target="_blank">
                                            🔗 ${this.extractFileName(link)}
                                        </a>
                                    `).join('')}
                                </div>
                            ` : ''}
                        </div>
                    `).join('') : '<p class="placeholder-text">No posts available</p>'}
                </div>
            </div>
        `;
    }
    
    /**
     * Bind events for game items
     */
    bindGameEvents() {
        const gameItems = this.gameListEl.querySelectorAll('.game-item');
        gameItems.forEach(item => {
            item.addEventListener('click', (e) => {
                const gameId = item.dataset.gameId;
                const index = parseInt(item.dataset.index);
                this.toggleGameExpansion(gameId, index);
            });
        });
    }
    
    /**
     * Toggle expansion of a game item
     */
    async toggleGameExpansion(gameId, index) {
        if (this.expandedGames.has(gameId)) {
            this.expandedGames.delete(gameId);
            this.renderGameList();
        } else {
            this.expandedGames.add(gameId);
            
            // Fetch detailed information for the game
            try {
                const gameDetail = await window.__TAURI__.invoke('fetch_game_detail', {
                    gameId: gameId
                });
                
                // Update the game with fetched details
                const game = this.currentGames[index];
                Object.assign(game, gameDetail);
                this.renderGameList();
                this.showDetailPanel(game);
            } catch (error) {
                console.error('Error fetching game details:', error);
            }
        }
    }
    
    /**
     * Show the detail panel for a game
     */
    showDetailPanel(game) {
        this.detailContentEl.innerHTML = `
            <div class="detail-content">
                <h3 class="detail-title">${this.escapeHtml(game.title)}</h3>
                <p class="detail-author">By ${this.escapeHtml(game.author)}</p>
                
                <div class="detail-stats">
                    <div class="stat-item">
                        <span class="stat-label">Total Posts</span>
                        <span class="stat-value">${game.posts?.length || 0}</span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">Releases</span>
                        <span class="stat-value">${game.releases?.length || 0}</span>
                    </div>
                    <div class="stat-item">
                        <span class="stat-label">Download Links</span>
                        <span class="stat-value">
                            ${game.releases?.reduce((sum, r) => sum + r.downloadLinks.length, 0) || 0}
                        </span>
                    </div>
                </div>
                
                ${game.releases && game.releases.length > 0 ? `
                    <div class="releases-section">
                        <h4>Releases</h4>
                        ${game.releases.map(release => `
                            <div class="release-card">
                                <span class="release-version">${release.version}</span>
                                <p>${this.escapeHtml(release.description)}</p>
                                <div class="release-downloads">
                                    ${release.downloadLinks.map(link => `
                                        <a href="${link.url}" class="download-link" target="_blank">
                                            🔗 ${link.fileName}
                                        </a>
                                    `).join('')}
                                </div>
                            </div>
                        `).join('')}
                    </div>
                ` : ''}
            </div>
        `;
        
        this.detailPanelEl.style.display = 'flex';
    }
    
    /**
     * Close the detail panel
     */
    closeDetailPanel() {
        this.detailPanelEl.style.display = 'none';
    }
    
    /**
     * Navigate to a specific page
     */
    async navigateToPage(pageIndex) {
        if (pageIndex < 0 || pageIndex >= 10) { // Assuming 10 pages max
            return;
        }
        
        this.currentGameIndex = pageIndex;
        await this.loadGames();
    }
    
    /**
     * Update pagination controls
     */
    updatePagination() {
        this.pageInfoEl.textContent = `Page ${this.currentGameIndex + 1}`;
        this.prevPageBtn.disabled = this.currentGameIndex === 0;
        this.nextPageBtn.disabled = this.currentGames.length < 20; // Assuming 20 games per page
    }
    
    /**
     * Show loading state
     */
    showLoadingState() {
        this.gameListEl.innerHTML = `
            <div class="loading-state">
                <div class="spinner"></div>
                <p>Loading games from cs.rin.ru...</p>
            </div>
        `;
    }
    
    /**
     * Render empty state
     */
    renderEmptyState() {
        this.gameListEl.innerHTML = `
            <div class="loading-state">
                <p>No games found. Try refreshing or check your connection.</p>
            </div>
        `;
    }
    
    /**
     * Show error message
     */
    showError(message) {
        this.gameListEl.innerHTML = `
            <div class="loading-state" style="color: #ef4444;">
                <p>${message}</p>
                <button class="btn btn-primary" onclick="location.reload()">Try Again</button>
            </div>
        `;
    }
    
    /**
     * Escape HTML to prevent XSS
     */
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
    
    /**
     * Extract file name from URL
     */
    extractFileName(url) {
        return url.split('/').filter(Boolean).pop();
    }
}

// Initialize the application when the DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    new CsRinRuApp();
});
