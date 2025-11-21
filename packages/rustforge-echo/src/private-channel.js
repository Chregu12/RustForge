/**
 * Private Channel
 *
 * Represents an authenticated private channel
 */
import Channel from './channel.js';

class PrivateChannel extends Channel {
    /**
     * Create a new PrivateChannel instance
     *
     * @param {Object} connector - WebSocket connector
     * @param {string} name - Channel name
     */
    constructor(connector, name) {
        super(connector, name);
    }

    /**
     * Subscribe to the channel
     *
     * Authenticates with the server before subscribing
     */
    async subscribe() {
        try {
            // Authenticate the channel subscription
            await this.connector.authorize(this.name);

            // Subscribe after successful authentication
            this.connector.subscribe(this.name);
            this.subscribed = true;
        } catch (error) {
            console.error(`Failed to subscribe to private channel ${this.name}:`, error);
            throw error;
        }
    }

    /**
     * Whisper an event to other channel members
     *
     * @param {string} event - Event name
     * @param {Object} data - Event data
     * @returns {PrivateChannel}
     */
    whisper(event, data) {
        this.connector.whisper(this.name, event, data);
        return this;
    }
}

export default PrivateChannel;
