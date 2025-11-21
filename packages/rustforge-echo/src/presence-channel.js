/**
 * Presence Channel
 *
 * Represents a presence channel that tracks who's online
 */
import PrivateChannel from './private-channel.js';

class PresenceChannel extends PrivateChannel {
    /**
     * Create a new PresenceChannel instance
     *
     * @param {Object} connector - WebSocket connector
     * @param {string} name - Channel name
     */
    constructor(connector, name) {
        super(connector, name);

        this.members = new Map();
        this.myID = null;
    }

    /**
     * Subscribe to the channel
     */
    async subscribe() {
        await super.subscribe();

        // Listen for presence events
        this.connector.on(this.name, 'presence:subscribed', (data) => {
            this.myID = data.me.id;
            this.members = new Map(Object.entries(data.members));

            if (this.hereCallback) {
                this.hereCallback(Array.from(this.members.values()));
            }
        });

        this.connector.on(this.name, 'presence:joining', (member) => {
            this.members.set(member.id, member);

            if (this.joiningCallback) {
                this.joiningCallback(member);
            }
        });

        this.connector.on(this.name, 'presence:leaving', (member) => {
            this.members.delete(member.id);

            if (this.leavingCallback) {
                this.leavingCallback(member);
            }
        });
    }

    /**
     * Handle initial member list
     *
     * @param {Function} callback - Callback with current members
     * @returns {PresenceChannel}
     */
    here(callback) {
        this.hereCallback = callback;
        return this;
    }

    /**
     * Handle member joining
     *
     * @param {Function} callback - Callback when a member joins
     * @returns {PresenceChannel}
     */
    joining(callback) {
        this.joiningCallback = callback;
        return this;
    }

    /**
     * Handle member leaving
     *
     * @param {Function} callback - Callback when a member leaves
     * @returns {PresenceChannel}
     */
    leaving(callback) {
        this.leavingCallback = callback;
        return this;
    }

    /**
     * Get all current members
     *
     * @returns {Array} Array of member objects
     */
    getMembers() {
        return Array.from(this.members.values());
    }

    /**
     * Check if a specific user is in the channel
     *
     * @param {string|number} userId - User ID
     * @returns {boolean}
     */
    isMember(userId) {
        return this.members.has(userId.toString());
    }
}

export default PresenceChannel;
