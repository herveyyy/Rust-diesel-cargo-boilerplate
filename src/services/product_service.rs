use uuid::Uuid;
use crate::database::models::Product;
use crate::usecases::product::{create_product, delete_product, get_all_products, get_product};

pub struct ProductService;

impl ProductService {
    pub fn create_product(name: &str) -> Result<Product, String> {
        create_product::execute(name)
    }

    pub fn get_product_by_id(id: Uuid) -> Result<Option<Product>, String> {
        get_product::execute(id)
    }

    pub fn get_all_products() -> Result<Vec<Product>, String> {
        get_all_products::execute()
    }

    pub fn delete_product(id: Uuid) -> Result<usize, String> {
        delete_product::execute(id)
    }
}
