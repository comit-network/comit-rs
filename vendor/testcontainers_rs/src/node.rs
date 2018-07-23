use api::{Container, Docker, Image};

pub struct Node<C, D: Docker> {
    _container: Container<D>,
    client: C,
}

pub trait ClientFactory<C> {
    fn new_client<D: Docker>(container: &Container<D>, image: &Self) -> C;
}

impl<T> ClientFactory<()> for T
where
    T: Image,
{
    fn new_client<D: Docker>(_container: &Container<D>, _image: &Self) -> () {
        ()
    }
}

impl<C, D: Docker> Node<C, D> {
    pub fn new<I: Image + ClientFactory<C>>() -> Node<C, D> {
        let docker = D::new();

        let image = I::default();

        let container = docker.run(&image);

        let client = I::new_client(&container, &image);

        Node {
            _container: container,
            client,
        }
    }

    pub fn get_client(&self) -> &C {
        &self.client
    }
}
