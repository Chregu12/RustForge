# RustForge Echo

JavaScript WebSocket client for RustForge broadcasting, inspired by Laravel Echo.

## Installation

```bash
npm install rustforge-echo
```

## Quick Start

```javascript
import Echo from 'rustforge-echo';

const echo = new Echo({
    broadcaster: 'websocket',
    host: 'localhost:8000',
    path: '/ws',
});

// Listen to public channel
echo.channel('posts')
    .listen('PostPublished', (event) => {
        console.log('New post:', event.post);
    });

// Listen to private channel
echo.private('user.1')
    .listen('MessageReceived', (event) => {
        console.log('New message:', event.message);
    });

// Join presence channel
echo.join('chat')
    .here((users) => {
        console.log('Users here:', users);
    })
    .joining((user) => {
        console.log('User joined:', user);
    })
    .leaving((user) => {
        console.log('User left:', user);
    })
    .listen('MessageSent', (event) => {
        console.log('Message:', event.message);
    });
```

## Configuration

```javascript
const echo = new Echo({
    broadcaster: 'websocket',
    host: 'localhost:8000',
    path: '/ws',
    authEndpoint: '/broadcasting/auth',
    auth: {
        headers: {
            'Authorization': 'Bearer YOUR_TOKEN'
        }
    }
});
```

## API

### Public Channels

```javascript
echo.channel('posts')
    .listen('PostPublished', (event) => {
        // Handle event
    })
    .stopListening('PostPublished')  // Stop listening
    .error((error) => {
        // Handle errors
    });
```

### Private Channels

```javascript
echo.private('user.1')
    .listen('MessageReceived', (event) => {
        // Handle event
    })
    .whisper('typing', { name: 'John' });  // Client-side event
```

### Presence Channels

```javascript
echo.join('chat')
    .here((users) => {
        // Initial user list
    })
    .joining((user) => {
        // User joined
    })
    .leaving((user) => {
        // User left
    })
    .listen('MessageSent', (event) => {
        // Regular events
    });
```

## Authentication

For private and presence channels, you need to set up an authentication endpoint:

```javascript
// Custom authorizer
const echo = new Echo({
    broadcaster: 'websocket',
    host: 'localhost:8000',
    authorizer: (channel) => {
        return fetch('/broadcasting/auth', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${token}`,
            },
            body: JSON.stringify({ channel_name: channel.name }),
        });
    }
});
```

## Browser Usage

```html
<script type="module">
import Echo from 'rustforge-echo';

const echo = new Echo({
    broadcaster: 'websocket',
    host: window.location.hostname + ':8000',
    path: '/ws',
});

echo.channel('notifications')
    .listen('NotificationSent', (e) => {
        console.log('Notification:', e);
    });
</script>
```

## Features

- ✅ Public channels
- ✅ Private channels with authentication
- ✅ Presence channels with member tracking
- ✅ Automatic reconnection
- ✅ Whisper events (client-side events)
- ✅ Error handling
- ✅ Laravel Echo compatible API

## License

MIT
