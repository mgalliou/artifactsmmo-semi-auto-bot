# \SimulationApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**fight_simulation_simulation_fight_simulation_post**](SimulationApi.md#fight_simulation_simulation_fight_simulation_post) | **POST** /simulation/fight_simulation | Fight Simulation



## fight_simulation_simulation_fight_simulation_post

> models::CombatSimulationResponseSchema fight_simulation_simulation_fight_simulation_post(combat_simulation_request_schema)
Fight Simulation

Simulate combat with fake characters against a monster multiple times. Member or founder account required.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**combat_simulation_request_schema** | [**CombatSimulationRequestSchema**](CombatSimulationRequestSchema.md) |  | [required] |

### Return type

[**models::CombatSimulationResponseSchema**](CombatSimulationResponseSchema.md)

### Authorization

[JWTBearer](../README.md#JWTBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

