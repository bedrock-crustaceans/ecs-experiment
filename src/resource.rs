pub trait Resource {}

pub struct Res<'a, R> {
    resource: &'a R,
}

pub struct ResMut<'a, R> {
    resource: &'a mut R,
}
