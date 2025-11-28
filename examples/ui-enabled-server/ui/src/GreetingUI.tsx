import { useState, useEffect } from 'react'
import './GreetingUI.css'

// MCP Client interface - will be provided by the host via window.mcp
interface MCPClient {
  callTool: (params: { name: string; arguments: Record<string, any> }) => Promise<any>
  getContext: () => Promise<any>
}

// Extend window interface
declare global {
  interface Window {
    mcp?: MCPClient
  }
}

export function GreetingUI() {
  const [name, setName] = useState('')
  const [greeting, setGreeting] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [isConnected, setIsConnected] = useState(false)
  const [context, setContext] = useState<any>(null)

  useEffect(() => {
    // Initialize MCP connection
    const initMCP = async () => {
      try {
        // Wait for MCP client to be available
        if (window.mcp) {
          setIsConnected(true)
          const ctx = await window.mcp.getContext()
          setContext(ctx)

          if (ctx?.tool?.arguments?.name) {
            setName(ctx.tool.arguments.name as string)
          }
        } else {
          // Retry after a short delay
          setTimeout(initMCP, 100)
        }
      } catch (err) {
        console.error('Failed to initialize MCP:', err)
      }
    }

    initMCP()
  }, [])

  const handleGreet = async () => {
    if (!window.mcp || !isConnected) {
      setError('Not connected to MCP host')
      return
    }

    if (!name.trim()) {
      setError('Please enter a name')
      return
    }

    setIsLoading(true)
    setError(null)

    try {
      const result = await window.mcp.callTool({
        name: 'greet_with_ui',
        arguments: { name: name.trim() }
      })

      if (result.isError) {
        setError('Failed to get greeting from server')
      } else if (result.content?.[0]?.type === 'text') {
        setGreeting(result.content[0].text)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred')
    } finally {
      setIsLoading(false)
    }
  }

  const handleReset = () => {
    setName('')
    setGreeting('')
    setError(null)
  }

  return (
    <div className="greeting-container">
      <div className="card">
        <header className="card-header">
          <h1>🎉 Interactive Greeting UI</h1>
          <div className="badges">
            <span className="badge badge-mcp">MCP Apps Extension</span>
            <span className={`badge ${isConnected ? 'badge-connected' : 'badge-disconnected'}`}>
              {isConnected ? '✓ Connected' : '✗ Disconnected'}
            </span>
          </div>
        </header>

        {context && (
          <div className="context-info">
            <h3>📋 Host Context</h3>
            <dl>
              <dt>Host:</dt>
              <dd>{context.hostInfo?.name || 'Unknown'} v{context.hostInfo?.version || 'N/A'}</dd>

              <dt>Theme:</dt>
              <dd>{context.theme || 'system'}</dd>

              <dt>Display Mode:</dt>
              <dd>{context.displayMode || 'inline'}</dd>

              {context.viewport && (
                <>
                  <dt>Viewport:</dt>
                  <dd>{context.viewport.width}x{context.viewport.height}px</dd>
                </>
              )}

              {context.locale && (
                <>
                  <dt>Locale:</dt>
                  <dd>{context.locale}</dd>
                </>
              )}

              {context.tool && (
                <>
                  <dt>Tool:</dt>
                  <dd>{context.tool.name}</dd>
                </>
              )}
            </dl>
          </div>
        )}

        <div className="greeting-form">
          <div className="form-group">
            <label htmlFor="name">Enter a name to greet:</label>
            <input
              id="name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && !isLoading && handleGreet()}
              placeholder="e.g., Alice"
              disabled={!isConnected || isLoading}
              className="name-input"
            />
          </div>

          <div className="button-group">
            <button
              onClick={handleGreet}
              disabled={!isConnected || isLoading || !name.trim()}
              className="btn btn-primary"
            >
              {isLoading ? '⏳ Loading...' : '👋 Say Hello'}
            </button>
            <button
              onClick={handleReset}
              disabled={isLoading}
              className="btn btn-secondary"
            >
              🔄 Reset
            </button>
          </div>

          {error && (
            <div className="alert alert-error">
              ⚠️ {error}
            </div>
          )}

          {greeting && (
            <div className="greeting-result">
              <h3>💬 Server Response:</h3>
              <p className="greeting-text">{greeting}</p>
            </div>
          )}
        </div>

        <footer className="card-footer">
          <div className="info-box">
            <strong>🔧 About this UI:</strong>
            <p>
              This is an interactive HTML/React interface served through the MCP Apps Extension (SEP-1865).
              It demonstrates bidirectional communication between the UI iframe and the MCP host using the{' '}
              <code>@mcp-ui/client</code> SDK.
            </p>
            <ul>
              <li>✅ Uses <code>useMCPClient()</code> React hook</li>
              <li>✅ Receives host context (theme, viewport, tool info)</li>
              <li>✅ Makes tool calls back to the server</li>
              <li>✅ Handles connection state and errors</li>
            </ul>
          </div>
        </footer>
      </div>
    </div>
  )
}
