# \MyAccountApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**change_password_my_change_password_post**](MyAccountApi.md#change_password_my_change_password_post) | **POST** /my/change_password | Change Password
[**get_account_details_my_details_get**](MyAccountApi.md#get_account_details_my_details_get) | **GET** /my/details | Get Account Details
[**get_bank_details_my_bank_get**](MyAccountApi.md#get_bank_details_my_bank_get) | **GET** /my/bank | Get Bank Details
[**get_bank_items_my_bank_items_get**](MyAccountApi.md#get_bank_items_my_bank_items_get) | **GET** /my/bank/items | Get Bank Items
[**get_ge_sell_history_my_grandexchange_history_get**](MyAccountApi.md#get_ge_sell_history_my_grandexchange_history_get) | **GET** /my/grandexchange/history | Get Ge Sell History
[**get_ge_sell_orders_my_grandexchange_orders_get**](MyAccountApi.md#get_ge_sell_orders_my_grandexchange_orders_get) | **GET** /my/grandexchange/orders | Get Ge Sell Orders



## change_password_my_change_password_post

> models::ResponseSchema change_password_my_change_password_post(change_password)
Change Password

Change your account password. Changing the password reset the account token.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**change_password** | [**ChangePassword**](ChangePassword.md) |  | [required] |

### Return type

[**models::ResponseSchema**](ResponseSchema.md)

### Authorization

[JWTBearer](../README.md#JWTBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_account_details_my_details_get

> models::MyAccountDetailsSchema get_account_details_my_details_get()
Get Account Details

Fetch account details.

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::MyAccountDetailsSchema**](MyAccountDetailsSchema.md)

### Authorization

[JWTBearer](../README.md#JWTBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_bank_details_my_bank_get

> models::BankResponseSchema get_bank_details_my_bank_get()
Get Bank Details

Fetch bank details.

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::BankResponseSchema**](BankResponseSchema.md)

### Authorization

[JWTBearer](../README.md#JWTBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_bank_items_my_bank_items_get

> models::DataPageSimpleItemSchema get_bank_items_my_bank_items_get(item_code, page, size)
Get Bank Items

Fetch all items in your bank.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**item_code** | Option<**String**> | Item to search in your bank. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageSimpleItemSchema**](DataPage_SimpleItemSchema_.md)

### Authorization

[JWTBearer](../README.md#JWTBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_ge_sell_history_my_grandexchange_history_get

> models::DataPageGeOrderHistorySchema get_ge_sell_history_my_grandexchange_history_get(id, code, page, size)
Get Ge Sell History

Fetch your sales history of the last 7 days.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | Option<**String**> | Order ID to search in your history. |  |
**code** | Option<**String**> | Item to search in your history. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageGeOrderHistorySchema**](DataPage_GeOrderHistorySchema_.md)

### Authorization

[JWTBearer](../README.md#JWTBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_ge_sell_orders_my_grandexchange_orders_get

> models::DataPageGeOrderSchema get_ge_sell_orders_my_grandexchange_orders_get(code, page, size)
Get Ge Sell Orders

Fetch your sell orders details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | Option<**String**> | The code of the item. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageGeOrderSchema**](DataPage_GEOrderSchema_.md)

### Authorization

[JWTBearer](../README.md#JWTBearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

