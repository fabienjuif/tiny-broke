# @tiny-broke/client

Javascript client for [tiny-broke](https://github.com/fabienjuif/tiny-broke).

## Service
```js
import connect from '@tiny-broke/client'

// connect to tiny-broke
const broke = connect(
  'invoices', // service name
  'tcp://localhost:3000', // tiny-broke uri
  true, // is it a worker? yes
)

// your service
const get = async (action) => {
  // this is here you are doing your stuff, like requesting your DB

  return {
    id: action.payload,
    price: 2342,
  }
}

// you register your `get` service behind `INVOICES>GET` event
// when tiny broke recieve the `INVOICES>GET` event it will call the logger callback first
broke.register(
  'INVOICES>GET', // register to this event type
  get, // calls this service to process the data and return a response
  (action) => console.log('getting invoice by id', JSON.stringify(action)), // log this when the event arrives (can be omitted)
)
```

## Client
```js
import connect from '@tiny-broke/client'

// connect to tiny-broke
const broke = connect(
  'graphql-api', // client name
  'tcp://localhost:3000', // tiny-broke uri
  false, // is it a worker? no this is a client
)

// event factory helper
const getInvoice = id => ({
  type: 'INVOICES>GET', // event type, must match a register from a service
  returnsType: `INVOICES>GET>${id}`, // this type is used to tells tiny-broke that we are waiting this response to complete
  payload: id, // whatever you want in payload, here the invoice id
})

const run = async (id) => {
  // ask a worker to process the `INVOICES>GET` event
  // and wait for the result
  const invoice = await broke.wait(getInvoice(id))

  // this is here you put your further processing
  console.log(invoice)
}

run(10)
```
