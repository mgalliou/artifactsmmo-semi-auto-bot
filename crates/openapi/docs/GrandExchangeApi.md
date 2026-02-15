# \GrandExchangeApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_ge_sell_history_grandexchange_history_code_get**](GrandExchangeApi.md#get_ge_sell_history_grandexchange_history_code_get) | **GET** /grandexchange/history/{code} | Get Ge Sell History
[**get_ge_sell_order_grandexchange_orders_id_get**](GrandExchangeApi.md#get_ge_sell_order_grandexchange_orders_id_get) | **GET** /grandexchange/orders/{id} | Get Ge Sell Order
[**get_ge_sell_orders_grandexchange_orders_get**](GrandExchangeApi.md#get_ge_sell_orders_grandexchange_orders_get) | **GET** /grandexchange/orders | Get Ge Sell Orders



## get_ge_sell_history_grandexchange_history_code_get

> models::DataPageGeOrderHistorySchema get_ge_sell_history_grandexchange_history_code_get(code, seller, buyer, page, size)
Get Ge Sell History

Fetch the sales history of the item for the last 7 days.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | **String** | The code of the item. | [required] |
**seller** | Option<**String**> | The seller (account name) of the item. |  |
**buyer** | Option<**String**> | The buyer (account name) of the item. |  |
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


## get_ge_sell_order_grandexchange_orders_id_get

> models::GeOrderResponseSchema get_ge_sell_order_grandexchange_orders_id_get(id)
Get Ge Sell Order

Retrieve the sell order of a item.

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


## get_ge_sell_orders_grandexchange_orders_get

> models::DataPageGeOrderSchema get_ge_sell_orders_grandexchange_orders_get(code, seller, page, size)
Get Ge Sell Orders

Fetch all sell orders.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | Option<**String**> | The code of the item. |  |
**seller** | Option<**String**> | The seller (account name) of the item. |  |
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

