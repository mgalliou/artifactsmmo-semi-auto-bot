# \GrandExchangeApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_ge_history_grandexchange_history_code_get**](GrandExchangeApi.md#get_ge_history_grandexchange_history_code_get) | **GET** /grandexchange/history/{code} | Get Ge History
[**get_ge_order_grandexchange_orders_id_get**](GrandExchangeApi.md#get_ge_order_grandexchange_orders_id_get) | **GET** /grandexchange/orders/{id} | Get Ge Order
[**get_ge_orders_grandexchange_orders_get**](GrandExchangeApi.md#get_ge_orders_grandexchange_orders_get) | **GET** /grandexchange/orders | Get Ge Orders



## get_ge_history_grandexchange_history_code_get

> models::DataPageGeOrderHistorySchema get_ge_history_grandexchange_history_code_get(code, account, page, size)
Get Ge History

Fetch the transaction history of the item for the last 7 days (buy and sell orders).

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | **String** | The code of the item. | [required] |
**account** | Option<**String**> | Account involved in the transaction (matches either seller or buyer). |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageGeOrderHistorySchema**](DataPage_GeOrderHistorySchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_ge_order_grandexchange_orders_id_get

> models::GeOrderResponseSchema get_ge_order_grandexchange_orders_id_get(id)
Get Ge Order

Retrieve a specific order by ID.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | The id of the order. | [required] |

### Return type

[**models::GeOrderResponseSchema**](GEOrderResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_ge_orders_grandexchange_orders_get

> models::DataPageGeOrderSchema get_ge_orders_grandexchange_orders_get(code, account, r#type, page, size)
Get Ge Orders

Fetch all orders (sell and buy orders).  Use the `type` parameter to filter by order type; when using `account`, `type` is required to decide whether to match seller or buyer.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | Option<**String**> | The code of the item. |  |
**account** | Option<**String**> | The account that sells or buys items. |  |
**r#type** | Option<[**models::GeOrderType**](Models__GeOrderType.md)> | Filter by order type (sell or buy). |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageGeOrderSchema**](DataPage_GEOrderSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

