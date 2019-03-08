const zmq = require('zeromq')
const serializeError = require('serialize-error')
const deserializeError = require('deserialize-error')

interface ZMQSocket {
  identity: string,
  connect: (uri: string) => void,
  close: () => void,
  on: (type: string, callback: (...args: Buffer[]) => void) => void,
  send: (message: Array<string | Buffer>) => void
}

const create = (name = '', uri: string, isWorker = false) => {
  let sock: ZMQSocket

  const sendResponse = (action: { type: string }) => {
    sock.send([action.type, '', JSON.stringify(action)])
  }

  interface Registration {
    type: string,
    callback: Function,
    log: Function,
  }
  const registrations = new Map<string, Registration>()

  const sendRegistrations = () => {
    if (!isWorker) return

    registrations.forEach((_, type) => {
      sock.send(['@@REGISTER', `@@ASKED>${type}`])
    })
  }

  const start = () => {
    if (sock) sock.close()
    sock = zmq.socket('dealer')
    sock.identity = `${isWorker ? 'worker' : 'client'}-${name}-${process.pid}` // FIXME: in a container all process id would be same ?? use uuid
    sock.connect(uri)

    sendRegistrations()
    ping()

    sock.on('message', async (_, messageBuffer) => {
      const message = messageBuffer.toString()

      // heart beating
      if (message === '@@PONG') {
        pongRecieved = true
        ping()
        return
      } else if (message === '@@REGISTER') {
        sendRegistrations()
        ping()
        return
      }

      // find the associated registrations
      const action = JSON.parse(message)
      const registration = registrations.get(action.type)
      if (!registration) {
        ping()
        return
      }

      // call the binded service
      const { log, callback } = registration
      let error
      let payload
      try {
        if (log) log(action)
        payload = await callback(action)
      } catch (ex) {
        console.error(`error while responding to ${action.type}`, ex)
        error = serializeError(ex)
      }

      // send reponse to broker
      sendResponse({
        ...action,
        type: action.returnsType,
        from: action.type,
        payload,
        error,
      })

      // reset heart beating
      ping()
    })
  }

  const PING_INTERVAL = 1000
  const PING_TIMEOUT = 1000
  let timerPing: NodeJS.Timeout
  let timerPong: NodeJS.Timeout
  let pongRecieved = false

  const recievePong = () => {
    if (timerPong) clearTimeout(timerPong)
    timerPong = setTimeout(
      () => {
        if (!pongRecieved) {
          console.log(`[${sock.identity}] trying to reconnect...`)
          start()
        }
      },
      PING_TIMEOUT,
    )
  }

  const ping = () => {
    if (timerPing) clearTimeout(timerPing)
    timerPing = setTimeout(
      () => {
        sock.send(['@@PING'])
        pongRecieved = false
        recievePong()
      },
      PING_INTERVAL
    )
  }

  const register = (type: string, callback: (action: { payload: any, error: string, from: string }) => void, log?: Function) => {
    registrations.set(
      type,
      {
        type,
        callback,
        log,
      },
    )

    if (isWorker) sock.send(['@@REGISTER', `@@ASKED>${type}`])
  }

  const wait = (action: { type: string, returnsType: string }) => {
    sock.send([`@@ASKED>${action.type}`, action.returnsType, JSON.stringify(action)])

    return new Promise((resolve, reject) => {
      register(
        action.returnsType,
        ({ payload, error, from }) => {
          if (!error) return resolve(payload)

          const thrownError = deserializeError(error)
          thrownError.from = from
          console.error(thrownError)

          return reject(thrownError)
        },
      )
    })
  }

  const close = () => {
    if (timerPing) clearTimeout(timerPing)
    if (timerPong) clearTimeout(timerPong)

    sock.close()
  }

  start()

  return {
    register,
    wait,
    close,
  }
}

export default create
