use lnrpc::*;

pub trait LightningRpcApi {
    type Err;

    fn add_invoice(&mut self, Invoice) -> Result<AddInvoiceResponse, Self::Err>;

    fn get_info(&mut self) -> Result<GetInfoResponse, Self::Err>;

    fn send_payment(&mut self, SendRequest) -> Result<SendResponse, Self::Err>;
}
