```bash
export resourceGroup="cloudresumerg"
export storageAccountName="cloudresumestorage2"

az storage account create \
  --name $storageAccountName \
  --resource-group $resourceGroup \
  --location eastus \
  --sku Standard_RAGRS \
  --kind StorageV2

az storage account show \
    --resource-group $resourceGroup \
    --name $storageAccountName \
    --query '[primaryEndpoints, secondaryEndpoints]'

az storage blob service-properties update --account-name $storageAccountName --static-website --404-document 404.html --index-document index.html

az storage blob upload-batch -s index.html -d '$web' --account-name $storageAccountName

export resourceGroup="cloudresumerg"
export location="eastus"
export suffix=$RANDOM*$RANDOM
export cosmosAccountName="cloudresume-$suffix"
export cosmosDatabaseName='VisitorCounter'

az group create --name $resourceGroup --location $location

az deployment create --resource-group $resourceGroup --template-file main.bicep --paramaters storageName=$storageName

az cosmosdb create \
    --resource-group $resourceGroup \
    --name $cosmosAccountName \
    --locations regionName=$location \
    --capabilities EnableTable

az cosmosdb table create \
    --account-name $cosmosAccountName \
    --resource-group $resourceGroup \
    --name $cosmosDatabaseName \
    --throughput 400

az cosmosdb keys list \
    --type connection-strings \
    --resource-group $resourceGroup \
    --name $cosmosAccountName \
    --query "connectionStrings[?description=='Primary Table Connection String'].connectionString" \
    --output tsv

az group delete --name $resourceGroup
```